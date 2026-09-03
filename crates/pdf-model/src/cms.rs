//! ISO 32000-2 §12.8.3.3's CMS signature value, read for the one thing it states in the clear.
//!
//! §12.8.1 divides verifying a signature into parts, and they do not need the same infrastructure:
//!
//! > The signer's certificate shall be determined and verified by the signature handler to match
//! > with any of the validation parameters and other conditions. If the verification fails, the
//! > signature shall be considered invalid. The digest shall be recomputed and compared with the
//! > one stored in the document. Differences between the two indicates that modifications have
//! > been made since the document was signed and thus the signature shall be considered invalid.
//!
//! The first sentence is a certificate, a chain and a trust decision. The last is **arithmetic
//! over bytes this program already holds** — and for the signature formats §12.8.3 defines, the
//! digest it names is written into the signature value where anyone can read it. This module
//! finds it. [`crate::signature::Integrity`] is what compares it.
//!
//! # What is read, and from where
//!
//! §12.8.3.3.1: "The CMS object shall conform to Internet RFC 5652". Of that structure this
//! module reads `SignedData`'s encapsulated content type, its certificate count, its one
//! `SignerInfo`'s digest algorithm, and the object identifiers of that signer's signed and
//! unsigned attributes — with the contents of `message-digest` (RFC 5652's
//! `id-messageDigest`), which is the digest of the signed content.
//!
//! **This paragraph used to end "[e]verything else in RFC 5652 is deliberately not read: the
//! certificates are X.509 and a trust decision, and the signature value itself needs the signer's
//! public key", and the three-hundred-and-ninety-second session made both halves obsolete.** The
//! certificates are handed over as values for [`crate::x509`] to read, and the signer's signature
//! algorithm, signature value and identifier are read here so that
//! [`crate::signature::Signature::authenticity`] can verify one. What is still not read is
//! anything a *trust* decision would need: no `crls`, no certification path, no validity dates.
//!
//! # What a matching digest proves, and what it does not
//!
//! **A mismatch is decisive and a match is not, on its own.** The digest sits beside the signature
//! rather than inside it, so anyone who alters the document can alter the recorded digest to match
//! — what they cannot do is make the *signature* over that digest verify. Checking that is
//! [`crate::signature::Signature::authenticity`]'s job, and the two answers are worth more
//! together than either is alone: four of the corpus's ten signatures verify over a digest their
//! documents no longer produce, which says the file was re-saved under a real signature. The
//! wording the program uses to a person keeps the two apart.

use crate::der::{DerError, INTEGER, OCTET_STRING, Reader, SEQUENCE, SET, Value};

/// How many of a signer's attributes are recorded.
///
/// RFC 5652 puts no bound on `SignedAttributes`, and this is the one allocation in the module that
/// a file's contents drive. §12.8.3.4.3 lists eleven attributes a `PAdES` signature may carry, so a
/// signature with more than sixty-four has stopped being one this reader has anything to say
/// about; the ones past the bound are dropped and [`SignedData::attributes_truncated`] says so.
const MAX_ATTRIBUTES: usize = 64;

/// How many certificates are kept out of a `SignedData`.
///
/// §12.8.3.3.1 requires one — "[a]t minimum the CMS object shall include the signer's X.509
/// signing certificate" — and permits a whole chain beside it. This bounds the one `Vec` a file's
/// contents size in this module; a signature carrying more has a certificate past the bound
/// ignored, which makes the signer *unmatched* rather than wrongly matched.
const MAX_CERTIFICATES: usize = 64;

/// RFC 5652's `id-signedData`, `1.2.840.113549.1.7.2`.
const ID_SIGNED_DATA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x02];
/// RFC 5652's `id-data`, `1.2.840.113549.1.7.1` — §12.8.3.4.3 (a)'s "id-data".
pub const ID_DATA: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x01];
/// RFC 5652's `id-contentType`, `1.2.840.113549.1.9.3`.
pub const ID_CONTENT_TYPE: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x03];
/// RFC 5652's `id-messageDigest`, `1.2.840.113549.1.9.4`.
pub const ID_MESSAGE_DIGEST: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x04];
/// RFC 5652's `id-signingTime`, `1.2.840.113549.1.9.5` — §12.8.3.4.3 (g)'s "signing-time".
pub const ID_SIGNING_TIME: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x05];
/// RFC 5652's `id-countersignature`, `1.2.840.113549.1.9.6` — §12.8.3.4.3 (i)'s
/// "counter-signature", which a `PAdES` signature "shall not" use.
pub const ID_COUNTERSIGNATURE: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x06];
/// RFC 3161's `id-ct-TSTInfo`, `1.2.840.113549.1.9.16.1.4` — what a document timestamp
/// encapsulates (§12.8.5).
pub const ID_CT_TST_INFO: &[u8] = &[
    0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x10, 0x01, 0x04,
];
/// §12.8.3.3.2's revocation information attribute, whose identifier the clause prints itself:
///
/// > adbe-revocationInfoArchival OBJECT IDENTIFIER::= {adbe(1.2.840.113583) acrobat(1)
/// > security(1) 8}
pub const ADBE_REVOCATION_INFO_ARCHIVAL: &[u8] =
    &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x2F, 0x01, 0x01, 0x08];

/// An X.501 `Name`'s encoding and a serial number's, which is how RFC 5652 names a certificate.
pub type IssuerAndSerial<'a> = (&'a [u8], &'a [u8]);

/// The public-key algorithm a `SignerInfo` says its signature was made with.
///
/// Table 260 names three families for a PDF signature — "RSA Algorithm Support", "DSA Algorithm
/// Support" and "ECDSA Algorithm Support ( defined by Internet RFC 5480 )" — and ISO/TS 32002
/// section 5.1.2 adds a fourth row to that table for `EdDSA`. All four are here.
///
/// What decides whether a signature is *verified* is the pair of this and the key, and for the
/// elliptic-curve families the second half carries more than this one does: every certificate on
/// every curve states the same `id-ecPublicKey`, so a curve this program does not compute on is
/// refused by its `namedCurve` identifier rather than by anything here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureAlgorithm<'a> {
    /// RSASSA-PKCS1-v1_5 — `rsaEncryption` or one of the `<hash>WithRSAEncryption` identifiers of
    /// RFC 8017's `pkcs-1` arc, which name the same padding and differ only in the digest that
    /// `digestAlgorithm` states anyway.
    ///
    /// `id-RSASSA-PSS` is deliberately **not** here: it is the same arc and a different padding,
    /// so treating it as this one would verify the wrong construction. It is [`Self::RsaPss`].
    RsaPkcs1V15,
    /// RSASSA-PSS — RFC 8017 Appendix A.2.3's `id-RSASSA-PSS`, the same key family under the
    /// salted padding [`crate::pss`] verifies.
    ///
    /// Which hash, mask generation function and salt length apply is not stated by the
    /// identifier: the `AlgorithmIdentifier`'s own parameters carry RFC 8017's
    /// `RSASSA-PSS-params`, which [`SignedData::signature_algorithm_parameters`] hands over and
    /// [`crate::pss::parameters`] reads.
    RsaPss,
    /// DSA — `id-dsa`, `id-dsa-with-sha1`, or one of the `id-dsa-with-sha2` arc's identifiers.
    ///
    /// Which digest was used is not read from here: `SignerInfo`'s own `digestAlgorithm` states
    /// it, and [`Digest`] is what reads that. See [`crate::dsa::is_dsa`].
    Dsa,
    /// ECDSA — `ecdsa-with-SHA*`, RFC 9688's `id-ecdsa-with-sha3-*`, or `id-ecPublicKey` itself.
    ///
    /// Which digest is not read from here either, for the reason [`crate::ecdsa::is_ecdsa`]
    /// states: ISO/TS 32002 section 5.1.4 requires the `SignerInfo`'s own `digestAlgorithm` to be
    /// the one "passed to the signature algorithm", so that is the one entry read.
    Ecdsa,
    /// `EdDSA` — RFC 8410's `id-Ed25519`, the row ISO/TS 32002 section 5.1.2 adds to Table 260.
    ///
    /// `id-Ed448` is deliberately *not* here: it is [`Self::Unrecognised`], so a file stating it
    /// is answered with that number rather than with a curve this program cannot compute on. See
    /// [`crate::eddsa`].
    EdDsa,
    /// Anything else, as the identifier the file wrote.
    Unrecognised(&'a [u8]),
}

impl<'a> SignatureAlgorithm<'a> {
    /// What an `AlgorithmIdentifier`'s object identifier names.
    ///
    /// The RSA identifiers are enumerated rather than matched by their arc, because
    /// `1.2.840.113549.1.1.10` is in the same arc and is RSASSA-PSS — a different padding, which a
    /// prefix test would silently verify as PKCS #1 v1.5.
    #[must_use]
    pub fn from_oid(oid: &'a [u8]) -> Self {
        if crate::dsa::is_dsa(oid) {
            return Self::Dsa;
        }
        if oid == crate::pss::ID_RSASSA_PSS {
            return Self::RsaPss;
        }
        if crate::ecdsa::is_ecdsa(oid) {
            return Self::Ecdsa;
        }
        if oid == crate::eddsa::ID_ED25519.as_bytes() {
            return Self::EdDsa;
        }
        match oid {
            // `pkcs-1` is 1.2.840.113549.1.1; the last octet is `rsaEncryption` (1) and the
            // `<hash>WithRSAEncryption` identifiers for MD2 (2), MD5 (4), SHA-1 (5), SHA-256 (11),
            // SHA-384 (12), SHA-512 (13) and SHA-224 (14).
            [
                0x2A,
                0x86,
                0x48,
                0x86,
                0xF7,
                0x0D,
                0x01,
                0x01,
                1 | 2 | 4 | 5 | 11 | 12 | 13 | 14,
            ] => Self::RsaPkcs1V15,
            other => Self::Unrecognised(other),
        }
    }
}

/// The digest algorithms ISO 32000-2's Table 260 and Table 256 name, with ISO/TS 32001's four.
///
/// Table 260 lists what each `/SubFilter` supports — "SHA1 ( PDF 1.3 ) SHA256 (PDF 1.6) SHA384
/// (PDF 1.7) SHA512 (PDF 1.7) RIPEMD160 (PDF 1.7 )" — and Table 256's `/DigestMethod` adds the
/// MD5 that was PDF 1.5's default. All six are here because a program that recognised five would
/// be silent about the sixth rather than wrong about it.
///
/// **Six is the base standard's list and is not the whole of it.** ISO/TS 32001:2022 section 5.1.1
/// states the addition — this document "adds support for digitally signing PDF documents using the
/// SHA3-256, SHA3-384, SHA3-512 and SHAKE256 hash algorithms in the secure hash algorithm 3 (SHA-3)
/// hash algorithm family as defined in FIPS PUB 202" — and its section 5.1.4 adds those four to
/// Table 260's Message Digest row.
///
/// **Table 256 is *not* one of the places they were added, and reading the errata is what says so**
/// (ADR 0390). ISO/TS 32001 section 5.1.3 did add the same four to Table 256's `/DigestMethod`, and
/// Errata Collection 3's issue #236 — `Review/Accepted`, in the copy under `doc/` — strikes that
/// subclause out with the instruction "Delete all of clause 5.1.3". So Table 256's `/DigestMethod`
/// keeps the base standard's list, and a sentence saying otherwise is quoting retired text. The
/// erratum lives in the PDF as an annotation, which is why no conversion shows it and
/// `tools/spec-errata` exists.
///
/// (Quoted in prose rather than as a blockquote for a reason worth knowing: a rustdoc blockquote is
/// checked verbatim against `doc/md/`'s ISO 32000-2 by `tools/conformance`, and words from another
/// document would be unattributable there. The quotation marks still mean verbatim.)
///
/// **Where they *are* added is part of the requirement**, which is why [`Self::ALL`] and
/// [`Self::TRIED_WHEN_UNSTATED`] are two lists rather than one: section 5.1.4 adds the values "to
/// the Message Digest value entry for adbe.pkcs7.detached, ETSI.CAdES.detached or ETSI.RFC3161",
/// three of Table 260's five `/SubFilter` columns, and says nothing about the other two.
///
/// All ten are computed here since ADR 0390. An identifier outside the ten is still reported by
/// its own number — [`crate::signature::Authenticity::UnknownDigest`] — rather than guessed at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Digest {
    /// MD5, Table 256's default for PDF 1.5 to 1.7 and deprecated with PDF 2.0.
    Md5,
    /// SHA-1, deprecated with PDF 2.0 and what four of the corpus's signatures use.
    Sha1,
    /// SHA-256.
    Sha256,
    /// SHA-384.
    Sha384,
    /// SHA-512.
    Sha512,
    /// RIPEMD-160, which Table 260 names for every `/SubFilter` and no corpus document writes.
    Ripemd160,
    /// SHA3-256, added by ISO/TS 32001:2022 section 5.1.4 as "SHA3-256 (PDF 2.x)".
    Sha3_256,
    /// SHA3-384, added by ISO/TS 32001:2022 section 5.1.4 as "SHA3-384 (PDF 2.x)".
    Sha3_384,
    /// SHA3-512, added by ISO/TS 32001:2022 section 5.1.4 as "SHA3-512 (PDF 2.x)".
    Sha3_512,
    /// SHAKE256 at [`SHAKE256_OCTETS`], on grounds the errata moved.
    ///
    /// **The published ISO/TS 32001:2022 pinned it**: section 5.1.4 said the algorithm "identified
    /// by the id-shake256 object identifier (OID) in section 2.3 of RFC 8419 shall be used", and
    /// its NOTE said what that bought — "[t]he requirement to use the id-shake256 OID fixes the
    /// SHAKE256 output length for the digest at 512 bits and serves to prohibit variable length
    /// SHAKE256 algorithm usage and prohibit use of SHAKE256 algorithms with OIDs other than
    /// id-shake256."
    ///
    /// **Errata Collection 3's issue #404 struck that sentence** — `Review/Accepted`, in the copy
    /// under `doc/` — and replaced it with a deferral: "used, the applicable stipulations on
    /// algorithm identifiers in RFC 8702, 3.1 and RFC 8419, 3.1, 3.2 shall be followed." **The NOTE
    /// was not struck**, so the only statement about an output length in any document this tree
    /// holds is now a NOTE describing a requirement that is no longer there. That is stranded text
    /// rather than a rule.
    ///
    /// So 512 bits is a **documented choice** and not a derivation (ADR 0390), and it is the
    /// narrow one: this variant is `id-shake256` squeezed to [`SHAKE256_OCTETS`], the reading both
    /// the retired sentence and the surviving NOTE agree on. Whatever else RFC 8702 section 3.1 and
    /// RFC 8419 sections 3.1 and 3.2 stipulate is unknown here, because this tree holds neither —
    /// and the cost of that is bounded in the safe direction: any other identifier, including any
    /// variable-length one those RFCs may define, is not in [`Self::from_oid`] and is reported by
    /// its own dotted decimal rather than computed at a guessed length.
    Shake256,
}

/// The octets [`Digest::Shake256`]'s output is squeezed to — "512 bits", ISO/TS 32001 section
/// 5.1.4's NOTE.
///
/// Named rather than written as 64 at the point of use because it is a *decision* and not a buffer
/// size: an extendable-output function has no natural length, this one was pinned by the published
/// text and unpinned by Errata Collection 3's issue #404, and [`Digest::Shake256`] carries the
/// argument for keeping the number the errata left standing in an unstruck NOTE.
pub const SHAKE256_OCTETS: usize = 512 / 8;

impl Digest {
    /// The algorithm an `AlgorithmIdentifier`'s object identifier names.
    ///
    /// `None` for anything else, which the caller reports rather than guessing at: a digest this
    /// program cannot compute is a question it cannot answer, and answering it with the wrong
    /// function would produce a mismatch that reads as a modified document.
    #[must_use]
    pub fn from_oid(oid: &[u8]) -> Option<Self> {
        // Seven of the ten are in the NIST arc (2.16.840.1.101.3.4.2.x) and differ only in their
        // last octet; SHA-1 is in the older OIW arc and MD5 and RIPEMD-160 in RSA's and
        // Teletrust's respectively. **The seven digits themselves come from a registry this tree
        // does not hold** — see the `identifiers` note on [`Self::oid`], which is where the cost
        // of that is written down.
        match oid {
            [0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x02, 0x05] => Some(Self::Md5),
            [0x2B, 0x0E, 0x03, 0x02, 0x1A] => Some(Self::Sha1),
            [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01] => Some(Self::Sha256),
            [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02] => Some(Self::Sha384),
            [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03] => Some(Self::Sha512),
            [0x2B, 0x24, 0x03, 0x02, 0x01] => Some(Self::Ripemd160),
            [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x08] => Some(Self::Sha3_256),
            [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x09] => Some(Self::Sha3_384),
            [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x0A] => Some(Self::Sha3_512),
            [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x0C] => Some(Self::Shake256),
            _ => None,
        }
    }

    /// The encoded object identifier of this algorithm — [`Self::from_oid`] the other way round.
    ///
    /// Needed because RFC 8017 section 9.2's `DigestInfo` puts the algorithm identifier *inside* the
    /// block a PKCS #1 v1.5 signature commits to, so verifying one means writing the identifier
    /// out again. The two functions are deliberately one pair of constants read in two directions:
    /// a second table would be a second thing to keep right.
    ///
    /// # The identifiers are transcribed, and this project cannot check them against a document
    ///
    /// **A deliberate decision with a cost, recorded here rather than taken quietly** (principle 1;
    /// ADR 0390). ISO 32000-2 and ISO/TS 32001 name these algorithms by *name* — "SHA3-256", and
    /// so on — and neither prints a digit of an object identifier. The one the standard is most
    /// specific about it names by symbol and defers: "the message digest algorithm identified by
    /// the id-shake256 object identifier (OID) in section 2.3 of RFC 8419 shall be used". So every
    /// number below is transcribed from a registry no document in `doc/` holds, exactly as the six
    /// that preceded it were.
    ///
    /// What limits the cost, and it is worth understanding rather than trusting:
    ///
    /// - **A transcription that is simply wrong costs a report and never a verdict.** An identifier
    ///   this table does not match falls out of [`Self::from_oid`] as `None`, which is reported by
    ///   its own dotted decimal — the behaviour a file stating that digest already got.
    /// - **A transcription that is wrong by *swapping two* would be a wrong answer**, so the pairing
    ///   is what the tests check: `x509::dotted` reads each constant back as digits, and — for the
    ///   three that have one — a second party's reading of the same registry is compared against
    ///   ours. Agreement raises confidence that the registry was read correctly, which is all
    ///   principle 5 ever lets another implementation do.
    /// - **SHAKE256 gained its second reading in the six-hundred-and-eighty-ninth session**, and
    ///   this bullet said it had none for a hundred and thirty-four sessions. The reason was true
    ///   and narrow — `shake` 0.1 publishes no identifier, where `sha3` does — and it stopped
    ///   deciding anything the moment `const-oid`'s `db` feature came into this crate for the
    ///   elliptic-curve family (ADR 0532): `const_oid::db::fips202::ID_SHAKE_256` is a second
    ///   party's reading of the same registry, at zero new packages, and the test below compares
    ///   all ten against it. **A silence about a second reading decays the way any claim about a
    ///   document does** — this one outlived its reason because nobody re-asked where else the
    ///   number might already be in the tree.
    #[must_use]
    pub fn oid(self) -> &'static [u8] {
        match self {
            Self::Md5 => &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x02, 0x05],
            Self::Sha1 => &[0x2B, 0x0E, 0x03, 0x02, 0x1A],
            Self::Sha256 => &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01],
            Self::Sha384 => &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02],
            Self::Sha512 => &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03],
            Self::Ripemd160 => &[0x2B, 0x24, 0x03, 0x02, 0x01],
            Self::Sha3_256 => &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x08],
            Self::Sha3_384 => &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x09],
            Self::Sha3_512 => &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x0A],
            // `id-shake256`, the identifier ISO/TS 32001 section 5.1.4 requires and does not print.
            Self::Shake256 => &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x0C],
        }
    }

    /// Every algorithm Table 260 or Table 256 names — the six, plus ISO/TS 32001 section 5.1.4's
    /// four.
    ///
    /// The complete set, in the order the standard introduced them. For the set a *caller* may try
    /// when a file states no algorithm at all, [`Self::TRIED_WHEN_UNSTATED`] is the narrower list
    /// and the difference is the standard's own.
    pub const ALL: [Self; 10] = [
        Self::Md5,
        Self::Sha1,
        Self::Sha256,
        Self::Sha384,
        Self::Sha512,
        Self::Ripemd160,
        Self::Sha3_256,
        Self::Sha3_384,
        Self::Sha3_512,
        Self::Shake256,
    ];

    /// The algorithms to try where the file states none, which is fewer than [`Self::ALL`].
    ///
    /// §12.8.3.2's `adbe.x509.rsa_sha1` records no digest algorithm anywhere a reader can see it —
    /// the identifier is inside the PKCS #1 block, under the key — so the only way to learn which
    /// a signature used is to build the block for each and compare. That is safe precisely because
    /// RFC 8017 section 8.2.2 compares whole blocks: six comparisons of a fixed-length string admit
    /// no more forgeries than one.
    ///
    /// **It is six rather than ten because ISO/TS 32001 says where its four go.** Section 5.1.4
    /// adds them "to the Message Digest value entry for adbe.pkcs7.detached, ETSI.CAdES.detached
    /// or ETSI.RFC3161" — three of Table 260's five `/SubFilter` columns, and this is one of the
    /// other two. Trying them here would be this program widening a table the standard did not.
    pub const TRIED_WHEN_UNSTATED: [Self; 6] = [
        Self::Md5,
        Self::Sha1,
        Self::Sha256,
        Self::Sha384,
        Self::Sha512,
        Self::Ripemd160,
    ];

    /// The algorithm's name, in the spelling Table 260 uses.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Md5 => "MD5",
            Self::Sha1 => "SHA1",
            Self::Sha256 => "SHA256",
            Self::Sha384 => "SHA384",
            Self::Sha512 => "SHA512",
            Self::Ripemd160 => "RIPEMD160",
            // ISO/TS 32001 section 5.1.4 spells these four, and the hyphen is theirs.
            Self::Sha3_256 => "SHA3-256",
            Self::Sha3_384 => "SHA3-384",
            Self::Sha3_512 => "SHA3-512",
            Self::Shake256 => "SHAKE256",
        }
    }

    /// The digest of a message given in pieces.
    ///
    /// Pieces rather than one slice because §12.8.1's byte range is "pairs of integers (starting
    /// byte offset, length in bytes)" describing a file with a hole in it, and joining them would
    /// copy the whole document to hash it. [`Self::hasher`] is the same computation for a message
    /// that is not in memory at all.
    #[must_use]
    pub fn compute(self, message: &[&[u8]]) -> Vec<u8> {
        let mut hasher = self.hasher();
        for piece in message {
            hasher.update(piece);
        }
        hasher.finish()
    }

    /// This function, started: fed the message a piece at a time and finished once.
    ///
    /// What lets a signature's `/ByteRange` be digested off a file on disk a window at a time
    /// (ADR 0812) rather than every signed byte being resident at once — which for a signed
    /// document is every byte of it but the hole.
    #[must_use]
    pub fn hasher(self) -> Hasher {
        use sha2::Digest as _;
        match self {
            Self::Md5 => Hasher::Md5(md5::Md5::new()),
            Self::Sha1 => Hasher::Sha1(sha1::Sha1::new()),
            Self::Sha256 => Hasher::Sha256(sha2::Sha256::new()),
            Self::Sha384 => Hasher::Sha384(sha2::Sha384::new()),
            Self::Sha512 => Hasher::Sha512(sha2::Sha512::new()),
            Self::Ripemd160 => Hasher::Ripemd160(ripemd::Ripemd160::new()),
            Self::Sha3_256 => Hasher::Sha3_256(sha3::Sha3_256::new()),
            Self::Sha3_384 => Hasher::Sha3_384(sha3::Sha3_384::new()),
            Self::Sha3_512 => Hasher::Sha3_512(sha3::Sha3_512::new()),
            Self::Shake256 => Hasher::Shake256(shake::Shake256::default()),
        }
    }
}

/// One of [`Digest`]'s functions in progress.
///
/// One variant per algorithm rather than a boxed trait object, so that the type says which ten
/// functions this program computes — the same ten [`Digest`] lists — and a hasher's cost is a
/// few hundred bytes of state on the stack rather than an allocation per window.
#[derive(Debug, Clone)]
pub enum Hasher {
    /// MD5 in progress.
    Md5(md5::Md5),
    /// SHA-1 in progress.
    Sha1(sha1::Sha1),
    /// SHA-256 in progress.
    Sha256(sha2::Sha256),
    /// SHA-384 in progress.
    Sha384(sha2::Sha384),
    /// SHA-512 in progress.
    Sha512(sha2::Sha512),
    /// RIPEMD-160 in progress.
    Ripemd160(ripemd::Ripemd160),
    /// SHA3-256 in progress.
    Sha3_256(sha3::Sha3_256),
    /// SHA3-384 in progress.
    Sha3_384(sha3::Sha3_384),
    /// SHA3-512 in progress.
    Sha3_512(sha3::Sha3_512),
    /// SHAKE256 in progress, to be squeezed to [`SHAKE256_OCTETS`].
    Shake256(shake::Shake256),
}

impl Hasher {
    /// Feeds the next piece of the message.
    pub fn update(&mut self, piece: &[u8]) {
        use sha2::Digest as _;
        match self {
            Self::Md5(hasher) => hasher.update(piece),
            Self::Sha1(hasher) => hasher.update(piece),
            Self::Sha256(hasher) => hasher.update(piece),
            Self::Sha384(hasher) => hasher.update(piece),
            Self::Sha512(hasher) => hasher.update(piece),
            Self::Ripemd160(hasher) => hasher.update(piece),
            Self::Sha3_256(hasher) => hasher.update(piece),
            Self::Sha3_384(hasher) => hasher.update(piece),
            Self::Sha3_512(hasher) => hasher.update(piece),
            Self::Shake256(hasher) => {
                use shake::Update as _;
                hasher.update(piece);
            }
        }
    }

    /// The digest of everything fed so far.
    #[must_use]
    pub fn finish(self) -> Vec<u8> {
        use sha2::Digest as _;
        match self {
            Self::Md5(hasher) => hasher.finalize().to_vec(),
            Self::Sha1(hasher) => hasher.finalize().to_vec(),
            Self::Sha256(hasher) => hasher.finalize().to_vec(),
            Self::Sha384(hasher) => hasher.finalize().to_vec(),
            Self::Sha512(hasher) => hasher.finalize().to_vec(),
            Self::Ripemd160(hasher) => hasher.finalize().to_vec(),
            Self::Sha3_256(hasher) => hasher.finalize().to_vec(),
            Self::Sha3_384(hasher) => hasher.finalize().to_vec(),
            Self::Sha3_512(hasher) => hasher.finalize().to_vec(),
            // The one arm that is not a `finalize`: SHAKE256 is an extendable-output function,
            // so it is squeezed to a length rather than finalised to one — and the length is
            // ISO/TS 32001 section 5.1.4's requirement rather than a property of the function,
            // which is the one thing a reader of this arm must not miss.
            Self::Shake256(hasher) => {
                use shake::{ExtendableOutput as _, XofReader as _};
                let mut out = vec![0; SHAKE256_OCTETS];
                hasher.finalize_xof().read(&mut out);
                out
            }
        }
    }
}

/// What stopped a signature value from being read as RFC 5652's `SignedData`.
///
/// Each of these is a fact about the file, and [`crate::signature::Integrity::Unreadable`] carries
/// it to a person rather than collapsing them all into a shrug.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CmsError {
    /// The ASN.1 encoding itself is malformed.
    #[error("the signature value is not well-formed ASN.1: {0}")]
    Encoding(#[from] DerError),
    /// The outermost value is not RFC 5652's `ContentInfo SEQUENCE`.
    #[error("the signature value is not a CMS ContentInfo")]
    NotContentInfo,
    /// `ContentInfo`'s content type is something other than `id-signedData`.
    ///
    /// §12.8.3.3.1 requires "a DER-encoded CMS binary data object containing the signature", and
    /// every `/SubFilter` in Table 260 that is not `adbe.x509.rsa_sha1` means `SignedData`.
    #[error("the signature value is a CMS object that is not SignedData")]
    NotSignedData,
    /// `SignedData` is there but its members are not in RFC 5652's order or shape.
    #[error("the signature value's SignedData is malformed")]
    MalformedSignedData,
    /// There is no `SignerInfo`, so nothing states a digest algorithm.
    ///
    /// §12.8.3.4.3 (d) requires exactly one for a `PAdES` signature; RFC 5652 permits a `SET OF`
    /// with none, which carries no signature at all.
    #[error("the signature value carries no signer")]
    NoSigner,
}

/// What §12.8.3.3's signature value says, as far as this program reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedData<'a> {
    /// `encapContentInfo`'s `eContentType`, as its encoded object identifier.
    pub content_type: &'a [u8],
    /// `encapContentInfo`'s `eContent`, where one is present.
    ///
    /// Absent for a detached signature, which is what `adbe.pkcs7.detached` and
    /// `ETSI.CAdES.detached` are named for. Present for `adbe.pkcs7.sha1`, where §12.8.3.3.1 says
    /// what it holds — "[t]he SHA-1 digest of the document's byte range shall be encapsulated in
    /// the CMS `SignedData` field with `ContentInfo` of type Data" — and for a document timestamp,
    /// where it is RFC 3161's `TSTInfo`.
    pub encapsulated: Option<&'a [u8]>,
    /// The entries of the `certificates [0] IMPLICIT` member, unparsed.
    ///
    /// §12.8.3.3.1: "[a]t minimum the CMS object shall include the signer's X.509 signing
    /// certificate. This certificate shall be used to verify the signature value in Contents ."
    /// So an empty list is a file failing that requirement, and [`crate::x509::read`] is what
    /// turns one of these into a key. **This was a `usize` until the three-hundred-and-ninety-
    /// second session**, which was every fact a program that could not verify a signature had.
    pub certificates: Vec<Value<'a>>,
    /// How many `SignerInfo`s the `signerInfos SET OF` holds.
    pub signers: usize,
    /// The first signer's `digestAlgorithm`, where this program knows the algorithm.
    pub digest: Option<Digest>,
    /// That algorithm's object identifier, whether or not it is one of the six.
    pub digest_algorithm: &'a [u8],
    /// The first signer's `signatureAlgorithm`, as its encoded object identifier.
    ///
    /// RFC 5652 makes this the algorithm "used by the signer", and [`SignatureAlgorithm`] is what
    /// this program does with it. Kept whether or not it is recognised, so that a signature this
    /// program cannot verify can still say by what number it declines.
    pub signature_algorithm: &'a [u8],
    /// That `AlgorithmIdentifier`'s parameters value, where the producer wrote one.
    ///
    /// For most algorithms the member is an ignorable `NULL`, and for `id-RSASSA-PSS` it is the
    /// whole point: RFC 8017 Appendix A.2.3 puts the scheme's hash, mask generation function,
    /// salt length and trailer field here, as `RSASSA-PSS-params` —
    /// [`crate::pss::parameters`] is the reader.
    pub signature_algorithm_parameters: Option<Value<'a>>,
    /// The first signer's `signature`, the octets that verification is over.
    pub signature: &'a [u8],
    /// The contents of the first signer's `signedAttrs [0] IMPLICIT`, where it states one.
    ///
    /// The *contents*, because RFC 5652 section 5.4 does not sign the bytes as they appear: "[a]
    /// separate encoding of the signedAttrs field is performed for message digest calculation.
    /// The IMPLICIT [0] tag in the signedAttrs is not used for the DER encoding, rather an
    /// EXPLICIT SET OF tag is used." [`SignedData::signed_attributes_encoding`] is that
    /// re-encoding.
    pub signed_attributes: Option<&'a [u8]>,
    /// The first signer's `sid`, where it is RFC 5652's `issuerAndSerialNumber`.
    ///
    /// The issuer `Name`'s contents and the serial number's, in that order — the pair a
    /// certificate is matched against by [`crate::x509::Certificate::is_named_by`].
    pub signer_issuer_and_serial: Option<IssuerAndSerial<'a>>,
    /// The first signer's `sid`, where it is instead `subjectKeyIdentifier [0]`.
    pub signer_key_identifier: Option<&'a [u8]>,
    /// The first signer's `message-digest` signed attribute — the digest of the signed content.
    pub message_digest: Option<&'a [u8]>,
    /// The object identifiers of the first signer's signed attributes, in the file's order.
    pub signed_attribute_types: Vec<&'a [u8]>,
    /// The same for its unsigned attributes.
    pub unsigned_attribute_types: Vec<&'a [u8]>,
    /// Whether either list stopped at [`MAX_ATTRIBUTES`] rather than at the file's end.
    pub attributes_truncated: bool,
}

impl<'a> SignedData<'a> {
    /// Whether the first signer states an attribute with this identifier.
    #[must_use]
    pub fn has_signed_attribute(&self, oid: &[u8]) -> bool {
        self.signed_attribute_types.contains(&oid)
    }

    /// Whether the first signer states an *unsigned* attribute with this identifier.
    #[must_use]
    pub fn has_unsigned_attribute(&self, oid: &[u8]) -> bool {
        self.unsigned_attribute_types.contains(&oid)
    }

    /// The bytes RFC 5652 section 5.4 says a signature over signed attributes is computed over.
    ///
    /// That clause requires "[t]he IMPLICIT [0] tag in the signedAttrs" not to be used for the DER
    /// encoding, "rather an EXPLICIT SET OF tag is used" — and it says which bytes go into the
    /// digest: "the DER encoding of the EXPLICIT SET OF tag, rather than of the IMPLICIT [0] tag,
    /// MUST be included in the message digest calculation along with the length and content octets
    /// of the `SignedAttributes` value."
    ///
    /// So the attributes are re-tagged as a `SET OF` — X.690's `31` in place of the `A0` the file
    /// wrote — with a definite length. The contents are the file's own bytes and are not
    /// re-ordered: DER requires a `SET OF`'s members to be sorted already, and re-sorting them
    /// here would change what a *non*-conforming producer signed and turn a verifiable signature
    /// into a failing one.
    ///
    /// `None` where the signer states no signed attributes, which RFC 5652 permits and which means
    /// the signature is over the content itself instead.
    #[must_use]
    pub fn signed_attributes_encoding(&self) -> Option<Vec<u8>> {
        let contents = self.signed_attributes?;
        let mut out = Vec::with_capacity(contents.len().saturating_add(6));
        out.push(SET);
        // X.690 clause 8.1.3: the short form below 128 octets, otherwise the long form with as
        // many length octets as the value needs. A signed-attributes set is a few hundred bytes.
        if contents.len() < 128 {
            out.push(u8::try_from(contents.len()).unwrap_or(0));
        } else {
            let length = contents.len().to_be_bytes();
            let significant = length
                .iter()
                .position(|&byte| byte != 0)
                .unwrap_or(length.len().saturating_sub(1));
            let octets = length.get(significant..).unwrap_or(&[]);
            out.push(0x80 | u8::try_from(octets.len()).unwrap_or(0));
            out.extend_from_slice(octets);
        }
        out.extend_from_slice(contents);
        Some(out)
    }

    /// The signer's public-key algorithm, as [`SignatureAlgorithm`] reads it.
    #[must_use]
    pub fn algorithm(&self) -> SignatureAlgorithm<'a> {
        SignatureAlgorithm::from_oid(self.signature_algorithm)
    }

    /// The digest RFC 3161's `TSTInfo` commits to, where this is a timestamp token.
    ///
    /// Table 255 states what a document timestamp's value is: "[t]he value of the messageImprint
    /// field within the `TimeStampToken` shall be a hash of the bytes of the document indicated by
    /// the `ByteRange`". `TSTInfo ::= SEQUENCE { version, policy, messageImprint, … }` and
    /// `MessageImprint ::= SEQUENCE { hashAlgorithm AlgorithmIdentifier, hashedMessage OCTET
    /// STRING }`, so the imprint is the third member's two parts.
    ///
    /// `None` where the encapsulated content is not a `TSTInfo`, where its algorithm is not one of
    /// the six, or where the encoding does not have that shape.
    #[must_use]
    pub fn timestamp_imprint(&self) -> Option<(Digest, &'a [u8])> {
        if self.content_type != ID_CT_TST_INFO {
            return None;
        }
        let mut reader = Reader::new(self.encapsulated?).ok()?;
        let info = reader.next_value().ok()??;
        let mut members = info.children().ok()?;
        // version INTEGER, policy OBJECT IDENTIFIER, messageImprint SEQUENCE.
        let _version = members.next_value().ok()??;
        let _policy = members.next_value().ok()??;
        let imprint = members.next_value().ok()??;
        let mut parts = imprint.children().ok()?;
        let algorithm = parts.next_value().ok()??;
        let hashed = parts.next_value().ok()??;
        if hashed.identifier != OCTET_STRING {
            return None;
        }
        let oid = algorithm.children().ok()?.next_value().ok()??;
        Some((Digest::from_oid(oid.object_identifier()?)?, hashed.contents))
    }
}

/// Reads a signature value as RFC 5652's `SignedData`.
///
/// # Errors
///
/// A [`CmsError`] naming what the value is instead. Nothing here is recovered from or guessed at:
/// a signature this reader cannot read is one it says it cannot read.
pub fn signed_data(bytes: &[u8]) -> Result<SignedData<'_>, CmsError> {
    let mut reader = Reader::new(bytes)?;
    let Some(content_info) = reader.next_value()? else {
        return Err(CmsError::NotContentInfo);
    };
    if content_info.identifier != SEQUENCE {
        return Err(CmsError::NotContentInfo);
    }
    let mut members = content_info.children()?;
    let Some(content_type) = members.next_value()? else {
        return Err(CmsError::NotContentInfo);
    };
    if content_type.object_identifier() != Some(ID_SIGNED_DATA) {
        return Err(CmsError::NotSignedData);
    }
    // `content [0] EXPLICIT ANY`: the tag wraps the SignedData rather than replacing its own.
    let Some(explicit) = members.next_value()? else {
        return Err(CmsError::MalformedSignedData);
    };
    if !explicit.is_context(0) {
        return Err(CmsError::MalformedSignedData);
    }
    let Some(signed) = explicit.children()?.next_value()? else {
        return Err(CmsError::MalformedSignedData);
    };
    read_signed_data(signed)
}

/// The members of `SignedData ::= SEQUENCE { version, digestAlgorithms, encapContentInfo,
/// certificates [0], crls [1], signerInfos }`.
///
/// Read by shape rather than by position, because `certificates` and `crls` are optional and a
/// positional reader would mistake an absent one for the member after it. The three that matter
/// each have a distinct tag at this level: `encapContentInfo` is the only `SEQUENCE`,
/// `certificates` the only `[0]`, and `signerInfos` the last `SET`.
fn read_signed_data(signed: Value<'_>) -> Result<SignedData<'_>, CmsError> {
    let mut members = signed.children()?;
    let mut content_type: &[u8] = &[];
    let mut encapsulated = None;
    let mut certificates = Vec::new();
    let mut signer_infos = None;
    while let Some(member) = members.next_value()? {
        if member.identifier == SEQUENCE && content_type.is_empty() {
            let mut parts = member.children()?;
            let Some(kind) = parts.next_value()? else {
                return Err(CmsError::MalformedSignedData);
            };
            let Some(kind) = kind.object_identifier() else {
                return Err(CmsError::MalformedSignedData);
            };
            content_type = kind;
            // `eContent [0] EXPLICIT OCTET STRING OPTIONAL`, so the octets are one level in.
            if let Some(wrapper) = parts.next_value()?
                && wrapper.is_context(0)
                && let Some(octets) = wrapper.children()?.next_value()?
                && octets.identifier == OCTET_STRING
            {
                encapsulated = Some(octets.contents);
            }
        } else if member.is_context(0) {
            let mut entries = member.children()?;
            while let Some(entry) = entries.next_value()? {
                if certificates.len() >= MAX_CERTIFICATES {
                    break;
                }
                certificates.push(entry);
            }
        } else if member.identifier == SET {
            signer_infos = Some(member);
        }
    }
    let Some(signer_infos) = signer_infos else {
        return Err(CmsError::NoSigner);
    };
    let mut signers = 0usize;
    let mut first = None;
    let mut entries = signer_infos.children()?;
    while let Some(entry) = entries.next_value()? {
        signers = signers.saturating_add(1);
        if first.is_none() {
            first = Some(entry);
        }
    }
    let Some(first) = first else {
        return Err(CmsError::NoSigner);
    };
    let parsed = read_signer_info(first)?;
    Ok(SignedData {
        content_type,
        encapsulated,
        certificates,
        signers,
        digest: Digest::from_oid(parsed.digest_algorithm),
        digest_algorithm: parsed.digest_algorithm,
        signature_algorithm: parsed.signature_algorithm,
        signature_algorithm_parameters: parsed.signature_algorithm_parameters,
        signature: parsed.signature,
        signed_attributes: parsed.signed_attributes,
        signer_issuer_and_serial: parsed.issuer_and_serial,
        signer_key_identifier: parsed.key_identifier,
        message_digest: parsed.message_digest,
        signed_attribute_types: parsed.signed_attribute_types,
        unsigned_attribute_types: parsed.unsigned_attribute_types,
        attributes_truncated: parsed.truncated,
    })
}

/// The parts of one `SignerInfo` this program reads.
struct Signer<'a> {
    digest_algorithm: &'a [u8],
    signature_algorithm: &'a [u8],
    signature_algorithm_parameters: Option<Value<'a>>,
    signature: &'a [u8],
    signed_attributes: Option<&'a [u8]>,
    issuer_and_serial: Option<IssuerAndSerial<'a>>,
    key_identifier: Option<&'a [u8]>,
    message_digest: Option<&'a [u8]>,
    signed_attribute_types: Vec<&'a [u8]>,
    unsigned_attribute_types: Vec<&'a [u8]>,
    truncated: bool,
}

/// `SignerInfo ::= SEQUENCE { version, sid, digestAlgorithm, signedAttrs [0] IMPLICIT OPTIONAL,
/// signatureAlgorithm, signature, unsignedAttrs [1] IMPLICIT OPTIONAL }`.
///
/// **Positional, and it has to be**: `sid`, `digestAlgorithm` and `signatureAlgorithm` are all
/// `SEQUENCE`s when the signer is identified by issuer and serial number, which is the ordinary
/// case and every one of the corpus's. So the digest algorithm is the third member — the shape
/// that a first attempt at this module got wrong, reading `sid` as the algorithm and finding no
/// digest at all. The two members after it are found the same way, with the one optional member
/// between them distinguished by its tag rather than by counting.
fn read_signer_info(info: Value<'_>) -> Result<Signer<'_>, CmsError> {
    let mut members = info.children()?;
    let Some(version) = members.next_value()? else {
        return Err(CmsError::MalformedSignedData);
    };
    if version.identifier != INTEGER {
        return Err(CmsError::MalformedSignedData);
    }
    let Some(sid) = members.next_value()? else {
        return Err(CmsError::MalformedSignedData);
    };
    let Some(algorithm) = members.next_value()? else {
        return Err(CmsError::MalformedSignedData);
    };
    let Some(oid) = algorithm.children()?.next_value()? else {
        return Err(CmsError::MalformedSignedData);
    };
    let Some(digest_algorithm) = oid.object_identifier() else {
        return Err(CmsError::MalformedSignedData);
    };
    let mut signer = Signer {
        digest_algorithm,
        signature_algorithm: &[],
        signature_algorithm_parameters: None,
        signature: &[],
        signed_attributes: None,
        issuer_and_serial: read_signer_identifier(sid)?,
        key_identifier: sid.is_context(0).then_some(sid.contents),
        message_digest: None,
        signed_attribute_types: Vec::new(),
        unsigned_attribute_types: Vec::new(),
        truncated: false,
    };
    // The three members that follow, in RFC 5652's order: an optional `[0]`, then the signature
    // algorithm and the signature, then an optional `[1]`.
    let mut next = members.next_value()?;
    if let Some(member) = next
        && member.is_context(0)
    {
        signer.signed_attributes = Some(member.contents);
        read_attributes(member, true, &mut signer)?;
        next = members.next_value()?;
    }
    if let Some(member) = next {
        let mut parts = member.children()?;
        if let Some(oid) = parts.next_value()?
            && let Some(identifier) = oid.object_identifier()
        {
            signer.signature_algorithm = identifier;
            // `AlgorithmIdentifier ::= SEQUENCE { algorithm, parameters ANY OPTIONAL }` — the
            // member after the identifier, kept for the one algorithm whose parameters decide
            // the whole scheme (RFC 8017's `RSASSA-PSS-params`).
            signer.signature_algorithm_parameters = parts.next_value()?;
        }
    }
    if let Some(member) = members.next_value()?
        && member.identifier == OCTET_STRING
    {
        signer.signature = member.contents;
    }
    while let Some(member) = members.next_value()? {
        if member.is_context(1) {
            read_attributes(member, false, &mut signer)?;
        }
    }
    Ok(signer)
}

/// `SignerIdentifier ::= CHOICE { issuerAndSerialNumber IssuerAndSerialNumber,
/// subjectKeyIdentifier [0] SubjectKeyIdentifier }`, in its first spelling.
///
/// `IssuerAndSerialNumber ::= SEQUENCE { issuer Name, serialNumber CertificateSerialNumber }`, and
/// both members are handed back as the contents the file wrote so that
/// [`crate::x509::Certificate::is_named_by`] compares encodings rather than decoding a name.
fn read_signer_identifier(sid: Value<'_>) -> Result<Option<IssuerAndSerial<'_>>, CmsError> {
    if sid.identifier != SEQUENCE {
        return Ok(None);
    }
    let mut parts = sid.children()?;
    let (Some(issuer), Some(serial)) = (parts.next_value()?, parts.next_value()?) else {
        return Ok(None);
    };
    if serial.identifier != INTEGER {
        return Ok(None);
    }
    Ok(Some((issuer.contents, serial.contents)))
}

/// One `SET OF Attribute`, recording every identifier and the `message-digest`'s value.
///
/// `[0]` is `signedAttrs` and `[1]` is `unsignedAttrs`; both are `SET OF Attribute`, and the
/// IMPLICIT tag replaces the SET's own.
fn read_attributes<'a>(
    member: Value<'a>,
    is_signed: bool,
    signer: &mut Signer<'a>,
) -> Result<(), CmsError> {
    let mut attributes = member.children()?;
    let mut names = Vec::new();
    while let Some(attribute) = attributes.next_value()? {
        if names.len() >= MAX_ATTRIBUTES {
            signer.truncated = true;
            break;
        }
        let mut parts = attribute.children()?;
        let Some(kind) = parts.next_value()? else {
            continue;
        };
        let Some(kind) = kind.object_identifier() else {
            continue;
        };
        names.push(kind);
        if is_signed && kind == ID_MESSAGE_DIGEST {
            // `AttributeValue ::= ANY`, in a `SET OF` of one: the digest is the octets of the
            // single value inside.
            if let Some(values) = parts.next_value()?
                && let Some(octets) = values.children()?.next_value()?
                && octets.identifier == OCTET_STRING
            {
                signer.message_digest = Some(octets.contents);
            }
        }
    }
    if is_signed {
        signer.signed_attribute_types = names;
    } else {
        signer.unsigned_attribute_types = names;
    }
    Ok(())
}

/// Hand-built signature values, shared with `signature.rs`'s tests.
///
/// **A fixture rather than a corpus document, and the reason is trap 8's — as far as the 974
/// go.** Four of the six signature formats §12.8.3 defines have no witness there — no document
/// timestamp, no `PAdES` signature, no `adbe.x509.rsa_sha1`, and nothing using four of Table 260's
/// six digests — so *that* corpus can rank none of them, and `tests/signatures.rs` confirms the
/// reading on the two it has over all ten of its signature dictionaries.
///
/// **The sentence above is a claim about a population and the population was one submodule, which
/// is the whole of what the eight-hundred-and-twenty-fifth session found (ADR 0754).** Run
/// `examples/signature_algorithm_census` over every document the tree holds instead, which is the
/// invocation `doc/todo/51` states, and all six formats have witnesses — `ETSI.RFC3161` and
/// `adbe.x509.rsa_sha1` among them, found in the crawl rather than in the submodule corpora. **No fixture here
/// is retired on that**, and the reason is the one trap 8 states from the other side: a witness
/// found in a crawl is a file nobody wrote for this purpose, so it can rank a format and cannot
/// *define* one. What it changes is which of these shapes the `cms` fuzz target now sees real
/// examples of, which is `fuzz/seed_cms.py`.
#[cfg(test)]
pub(crate) mod fixtures {
    use super::{
        Digest, ID_CONTENT_TYPE, ID_CT_TST_INFO, ID_DATA, ID_MESSAGE_DIGEST, ID_SIGNING_TIME,
    };

    /// A DER `SEQUENCE`, `SET` or context tag around already-encoded children.
    fn tagged(identifier: u8, children: &[Vec<u8>]) -> Vec<u8> {
        primitive(identifier, &children.concat())
    }

    /// One value with the given identifier and contents, in X.690's shortest length form.
    ///
    /// Two length forms are spelled, and the second arrived with the DSA fixture: a real
    /// certificate is over a thousand octets, so `0x82` and two length octets are reachable and
    /// the assertion below is where a third form would announce itself.
    fn primitive(identifier: u8, contents: &[u8]) -> Vec<u8> {
        let mut out = vec![identifier];
        if contents.len() < 128 {
            out.push(u8::try_from(contents.len()).unwrap_or(0));
        } else if contents.len() < 256 {
            out.push(0x81);
            out.push(u8::try_from(contents.len()).unwrap_or(0));
        } else {
            assert!(contents.len() < 65536, "the fixtures are all short");
            out.push(0x82);
            #[expect(
                clippy::cast_possible_truncation,
                reason = "the assertion above is what bounds this to two octets"
            )]
            out.extend_from_slice(&(contents.len() as u16).to_be_bytes());
        }
        out.extend_from_slice(contents);
        out
    }

    /// The `AlgorithmIdentifier` for one digest, with no parameters member.
    fn digest_algorithm(digest: Digest) -> Vec<u8> {
        tagged(0x30, &[primitive(0x06, digest.oid())])
    }

    /// The `AlgorithmIdentifier` for SHA-256.
    fn sha256_algorithm() -> Vec<u8> {
        digest_algorithm(Digest::Sha256)
    }

    /// One signed attribute: `SEQUENCE { OID, SET OF value }`.
    fn attribute(oid: &[u8], value: Vec<u8>) -> Vec<u8> {
        tagged(0x30, &[primitive(0x06, oid), tagged(0x31, &[value])])
    }

    /// One `SignerInfo` stating `digest` as its `digestAlgorithm`, with the signed attributes given.
    fn signer(digest: Digest, attributes: Option<Vec<Vec<u8>>>) -> Vec<u8> {
        let mut members = vec![
            primitive(0x02, &[0x01]),                  // version
            tagged(0x30, &[primitive(0x02, &[0x2A])]), // sid: issuer and serial number
            digest_algorithm(digest),                  // digestAlgorithm
        ];
        if let Some(attributes) = attributes {
            members.push(tagged(0xA0, &attributes));
        }
        members.push(sha256_algorithm()); // signatureAlgorithm
        members.push(primitive(0x04, &[0xDE, 0xAD])); // signature
        tagged(0x30, &members)
    }

    /// `ContentInfo { id-signedData, [0] SignedData }` around one signer.
    fn content_info(
        digest: Digest,
        content_type: &[u8],
        encapsulated: Option<&[u8]>,
        signer: Vec<u8>,
    ) -> Vec<u8> {
        let mut encapsulated_info = vec![primitive(0x06, content_type)];
        if let Some(content) = encapsulated {
            encapsulated_info.push(tagged(0xA0, &[primitive(0x04, content)]));
        }
        let body = tagged(
            0x30,
            &[
                primitive(0x02, &[0x01]),
                tagged(0x31, &[digest_algorithm(digest)]),
                tagged(0x30, &encapsulated_info),
                // Two certificates, stood in for by empty sequences: this program counts them
                // and reads none, so their contents would be a fiction with no reader.
                tagged(0xA0, &[primitive(0x30, &[0x00]), primitive(0x30, &[0x00])]),
                tagged(0x31, &[signer]),
            ],
        );
        tagged(
            0x30,
            &[
                primitive(
                    0x06,
                    &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x02],
                ),
                tagged(0xA0, &[body]),
            ],
        )
    }

    /// A detached `adbe.pkcs7.detached` signature value, with a message digest of `digest`.
    ///
    /// Carries a `signing-time` attribute as well, because §12.8.3.4.2 forbids stating that
    /// *and* the signature dictionary's `/M`, and a fixture with only one of the two could not
    /// exercise the rule.
    pub(crate) fn detached(digest: &[u8]) -> Vec<u8> {
        detached_stating(Digest::Sha256, digest)
    }

    /// The same, stating an algorithm of the caller's choosing rather than SHA-256.
    ///
    /// Exists for ISO/TS 32001's four (ADR 0390), which no document in the population states, so a
    /// hand-built value is the only way to carry one through `Signature::integrity` end to end
    /// (trap 8). The `/SubFilter` its caller pairs it with matters: section 5.1.4 adds the four
    /// only for `adbe.pkcs7.detached`, `ETSI.CAdES.detached` and `ETSI.RFC3161`, and this fixture
    /// is the first of those three.
    pub(crate) fn detached_stating(algorithm: Digest, digest: &[u8]) -> Vec<u8> {
        content_info(
            algorithm,
            ID_DATA,
            None,
            signer(
                algorithm,
                Some(vec![
                    attribute(ID_CONTENT_TYPE, primitive(0x06, ID_DATA)),
                    attribute(ID_SIGNING_TIME, primitive(0x17, b"260807000000Z")),
                    attribute(ID_MESSAGE_DIGEST, primitive(0x04, digest)),
                ]),
            ),
        )
    }

    /// The same with no signed attributes at all, which RFC 5652 permits.
    ///
    /// `bug854315.pdf` is this shape. There is then no `message-digest` recording the document's
    /// digest, so the only thing that could answer question 1 is question 2's public key.
    pub(crate) fn without_signed_attributes() -> Vec<u8> {
        content_info(Digest::Sha256, ID_DATA, None, signer(Digest::Sha256, None))
    }

    /// An `adbe.pkcs7.sha1` value, which encapsulates the document's digest rather than
    /// detaching from it.
    ///
    /// §12.8.3.3.1: "[t]he SHA-1 digest of the document's byte range shall be encapsulated in the
    /// CMS `SignedData` field with `ContentInfo` of type Data. The digest of that `SignedData`
    /// shall be incorporated as the normal CMS digest." So the `message-digest` attribute here is
    /// a digest *of the digest*, and it is written deliberately wrong: a reader that reached for
    /// it would compare the wrong two values and report a document that had not changed as one
    /// that had.
    pub(crate) fn encapsulating(digest: &[u8]) -> Vec<u8> {
        content_info(
            Digest::Sha256,
            ID_DATA,
            Some(digest),
            signer(
                Digest::Sha256,
                Some(vec![
                    attribute(ID_CONTENT_TYPE, primitive(0x06, ID_DATA)),
                    attribute(ID_MESSAGE_DIGEST, primitive(0x04, &[0xFF; 32])),
                ]),
            ),
        )
    }

    /// A detached signature whose signer states DSA, over a real certificate and a real signature.
    ///
    /// **The one fixture built from something other than made-up octets, and it has to be**: this
    /// is what carries Table 260's second algorithm family through
    /// [`crate::signature::Signature::authenticity`] end to end — the algorithm match, the
    /// certificate lookup by issuer and serial number, [`crate::x509`]'s reading of a `Dss-Parms`
    /// key, and [`crate::dsa`]'s arithmetic. No corpus document does any of that (see
    /// `crate::dsa`'s fixtures for the population), so a hand-built pair is the only witness there
    /// is.
    ///
    /// The signer states **no signed attributes**, which RFC 5652 permits and which makes the
    /// signature one over the content itself — for a detached signature, the byte range. That is
    /// what lets a fixture use a signature made once over bytes chosen here: with signed
    /// attributes the digest would have to be in the attributes that are themselves signed, and
    /// nothing in this tree holds the private key to close that circle.
    pub(crate) fn detached_dsa(
        certificate: &[u8],
        issuer: &[u8],
        serial: &[u8],
        signature: &[u8],
    ) -> Vec<u8> {
        // `SignerInfo`, positionally: version, sid, digestAlgorithm, signatureAlgorithm, signature.
        let signer = tagged(
            0x30,
            &[
                primitive(0x02, &[0x01]),
                tagged(
                    0x30,
                    &[tagged(0x30, &[issuer.to_vec()]), primitive(0x02, serial)],
                ),
                sha256_algorithm(),
                // `id-dsa-with-sha256`, the identifier RFC 5758 section 3.1 assigns.
                tagged(
                    0x30,
                    &[primitive(
                        0x06,
                        &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x03, 0x02],
                    )],
                ),
                primitive(0x04, signature),
            ],
        );
        let body = tagged(
            0x30,
            &[
                primitive(0x02, &[0x01]),
                tagged(0x31, &[sha256_algorithm()]),
                tagged(0x30, &[primitive(0x06, ID_DATA)]),
                tagged(0xA0, &[certificate.to_vec()]),
                tagged(0x31, &[signer]),
            ],
        );
        tagged(
            0x30,
            &[
                primitive(
                    0x06,
                    &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x02],
                ),
                tagged(0xA0, &[body]),
            ],
        )
    }

    /// A detached signature whose signer states `id-RSASSA-PSS`, over a real certificate and a
    /// real PSS signature.
    ///
    /// The counterpart of [`detached_dsa`] for the RSA family's *other* padding, and what
    /// carries it through [`crate::signature::Signature::authenticity`] end to end: the
    /// algorithm identifier recognised as PSS rather than as PKCS #1 v1.5, RFC 8017 Appendix
    /// A.2.3's `RSASSA-PSS-params` read out of the `signatureAlgorithm`'s own parameters —
    /// SHA-256, MGF1 with SHA-256, a 32-octet salt, trailer field 1, spelled out rather than
    /// defaulted because the six real PSS signatures in the `SafeDocs` population spell theirs —
    /// the certificate lookup by issuer and serial number, and [`crate::pss`]'s arithmetic.
    ///
    /// The signer states **no signed attributes**, for [`detached_dsa`]'s reason: RFC 5652 then
    /// signs the content itself — the byte range — which is what lets a fixture use a signature
    /// made once over bytes chosen here.
    pub(crate) fn detached_pss(
        certificate: &[u8],
        issuer: &[u8],
        serial: &[u8],
        signature: &[u8],
    ) -> Vec<u8> {
        // RSASSA-PSS-params, each member in its explicit context tag.
        let mgf1_oid = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x08];
        let parameters = tagged(
            0x30,
            &[
                tagged(0xA0, &[sha256_algorithm()]),
                tagged(
                    0xA1,
                    &[tagged(
                        0x30,
                        &[primitive(0x06, mgf1_oid), sha256_algorithm()],
                    )],
                ),
                tagged(0xA2, &[primitive(0x02, &[32])]),
                tagged(0xA3, &[primitive(0x02, &[1])]),
            ],
        );
        let pss_algorithm = tagged(
            0x30,
            &[
                primitive(
                    0x06,
                    &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0A],
                ),
                parameters,
            ],
        );
        // `SignerInfo`, positionally: version, sid, digestAlgorithm, signatureAlgorithm,
        // signature.
        let signer = tagged(
            0x30,
            &[
                primitive(0x02, &[0x01]),
                tagged(
                    0x30,
                    &[tagged(0x30, &[issuer.to_vec()]), primitive(0x02, serial)],
                ),
                sha256_algorithm(),
                pss_algorithm,
                primitive(0x04, signature),
            ],
        );
        let body = tagged(
            0x30,
            &[
                primitive(0x02, &[0x01]),
                tagged(0x31, &[sha256_algorithm()]),
                tagged(0x30, &[primitive(0x06, ID_DATA)]),
                tagged(0xA0, &[certificate.to_vec()]),
                tagged(0x31, &[signer]),
            ],
        );
        tagged(
            0x30,
            &[
                primitive(
                    0x06,
                    &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x02],
                ),
                tagged(0xA0, &[body]),
            ],
        )
    }

    /// A detached signature whose signer states an elliptic-curve algorithm.
    ///
    /// The counterpart of [`detached_dsa`] for the two families ISO/TS 32002 governs, and what
    /// carries them through [`crate::signature::Signature::authenticity`] end to end: the
    /// `signatureAlgorithm` recognised, the certificate found by RFC 5652's issuer and serial
    /// number, the `namedCurve` read out of its `subjectPublicKeyInfo`, and the arithmetic.
    ///
    /// `algorithm` is the `signatureAlgorithm`'s object identifier — `ecdsa-with-SHA*` or
    /// `id-Ed25519` — and `digest` is what the `SignerInfo` states, which ISO/TS 32002 section
    /// 5.1.4 requires to be the one the signature was made under.
    ///
    /// The signer states **no signed attributes**, for [`detached_dsa`]'s reason: RFC 5652 then
    /// signs the content itself — the byte range — which is what lets a fixture use a signature
    /// made once over bytes chosen by the test.
    pub(crate) fn detached_curve(
        certificate: &[u8],
        issuer: &[u8],
        serial: &[u8],
        digest: Digest,
        algorithm: &[u8],
        signature: &[u8],
    ) -> Vec<u8> {
        // `SignerInfo`, positionally: version, sid, digestAlgorithm, signatureAlgorithm, signature.
        let signer = tagged(
            0x30,
            &[
                primitive(0x02, &[0x01]),
                tagged(
                    0x30,
                    &[tagged(0x30, &[issuer.to_vec()]), primitive(0x02, serial)],
                ),
                digest_algorithm(digest),
                tagged(0x30, &[primitive(0x06, algorithm)]),
                primitive(0x04, signature),
            ],
        );
        let body = tagged(
            0x30,
            &[
                primitive(0x02, &[0x01]),
                tagged(0x31, &[digest_algorithm(digest)]),
                tagged(0x30, &[primitive(0x06, ID_DATA)]),
                tagged(0xA0, &[certificate.to_vec()]),
                tagged(0x31, &[signer]),
            ],
        );
        tagged(
            0x30,
            &[
                primitive(
                    0x06,
                    &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x02],
                ),
                tagged(0xA0, &[body]),
            ],
        )
    }

    /// An `ETSI.RFC3161` timestamp token committing to `digest`.
    ///
    /// `TSTInfo ::= SEQUENCE { version, policy, messageImprint, … }`, encapsulated as
    /// `id-ct-TSTInfo`. Table 255: "[t]he value of the messageImprint field within the
    /// `TimeStampToken` shall be a hash of the bytes of the document indicated by the `ByteRange`".
    pub(crate) fn timestamp_token(digest: &[u8]) -> Vec<u8> {
        let info = tagged(
            0x30,
            &[
                primitive(0x02, &[0x01]),       // version
                primitive(0x06, &[0x2A, 0x03]), // policy
                tagged(
                    0x30,
                    &[sha256_algorithm(), primitive(0x04, digest)], // messageImprint
                ),
            ],
        );
        content_info(
            Digest::Sha256,
            ID_CT_TST_INFO,
            Some(&info),
            signer(Digest::Sha256, None),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::detached;
    use super::{
        CmsError, Digest, ID_DATA, ID_MESSAGE_DIGEST, ID_SIGNING_TIME, SHAKE256_OCTETS, signed_data,
    };

    /// A DER `SEQUENCE` around already-encoded children, for the two malformed fixtures below.
    fn tagged(identifier: u8, children: &[Vec<u8>]) -> Vec<u8> {
        primitive(identifier, &children.concat())
    }

    /// One value with the given identifier and contents, in X.690's short length form.
    fn primitive(identifier: u8, contents: &[u8]) -> Vec<u8> {
        let mut out = vec![identifier];
        assert!(contents.len() < 128, "the malformed fixtures are short");
        out.push(u8::try_from(contents.len()).unwrap_or(0));
        out.extend_from_slice(contents);
        out
    }

    /// The one thing this module exists to find, found.
    #[test]
    fn a_detached_signature_states_its_algorithm_and_its_message_digest() {
        let expected = Digest::Sha256.compute(&[b"the signed bytes"]);
        let bytes = detached(&expected);
        let cms = signed_data(&bytes).expect("a SignedData");
        assert_eq!(cms.digest, Some(Digest::Sha256));
        assert_eq!(cms.message_digest, Some(expected.as_slice()));
        assert_eq!(cms.content_type, ID_DATA, "§12.8.3.4.3 (a)'s id-data");
        assert_eq!(cms.encapsulated, None, "detached: no encapsulated content");
        assert_eq!(cms.signers, 1, "§12.8.3.4.3 (d)'s one SignerInfo");
        assert_eq!(cms.certificates.len(), 2);
        assert!(cms.has_signed_attribute(ID_MESSAGE_DIGEST));
        assert!(cms.has_signed_attribute(ID_SIGNING_TIME));
        assert!(!cms.has_unsigned_attribute(ID_MESSAGE_DIGEST));
        assert!(!cms.attributes_truncated);
    }

    /// `digestAlgorithm` is the *third* member of a `SignerInfo`, not the first `SEQUENCE`.
    ///
    /// `sid` is a `SEQUENCE` too whenever the signer is named by issuer and serial number, which
    /// is every signature in the corpus. Reading by shape rather than by position finds the
    /// issuer's `SEQUENCE` and reports no digest at all — a wrong answer wearing the shape of an
    /// unsupported one, which is the failure this test pins.
    #[test]
    fn the_signers_own_sequence_is_not_mistaken_for_its_digest_algorithm() {
        let bytes = detached(&[0x00; 32]);
        let cms = signed_data(&bytes).expect("a SignedData");
        assert_eq!(
            cms.digest_algorithm,
            [0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01],
            "the third member's algorithm identifier and not the second's"
        );
    }

    /// The base standard's six, and one identifier the tables do not name.
    #[test]
    fn the_digest_algorithms_are_the_ones_the_tables_name() {
        assert_eq!(
            Digest::from_oid(&[0x2B, 0x0E, 0x03, 0x02, 0x1A]),
            Some(Digest::Sha1)
        );
        assert_eq!(
            Digest::from_oid(&[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x02, 0x05]),
            Some(Digest::Md5)
        );
        assert_eq!(
            Digest::from_oid(&[0x2B, 0x24, 0x03, 0x02, 0x01]),
            Some(Digest::Ripemd160)
        );
        assert_eq!(Digest::from_oid(&[0x2A, 0x03]), None);

        // Published vectors, so that the six names are attached to the six functions rather than
        // to whichever crate happened to be listed beside them.
        assert_eq!(
            hex(&Digest::Sha1.compute(&[b"abc"])),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            hex(&Digest::Sha256.compute(&[b"ab", b"c"])),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            "and the pieces are hashed as one message"
        );
        assert_eq!(
            hex(&Digest::Md5.compute(&[b"abc"])),
            "900150983cd24fb0d6963f7d28e17f72"
        );
        assert_eq!(
            hex(&Digest::Ripemd160.compute(&[b"abc"])),
            "8eb208f7e05d987a9b044a8e98c6b087f15a0bfc"
        );
    }

    /// ISO/TS 32001 section 5.1.4's four, against the example values their publisher states.
    ///
    /// **The vectors are NIST's own**, from the example-value documents that accompany FIPS PUB 202
    /// — which is the publication ISO/TS 32001 section 5.1.1 names as where these algorithms are
    /// "defined" — for the empty message and for its 1600-bit message, 200 octets of `0xA3`. The
    /// second is worth as much as the first: it is longer than every one of the four rates, so it
    /// is the case that absorbs more than one block, and splitting it here also pins that pieces
    /// are hashed as one message.
    ///
    /// No corpus document states any of these four (`signature_algorithm_census` over 67 460
    /// documents), so trap 8 applies and a published vector is the only witness there can be.
    #[test]
    fn iso_ts_32001s_four_digests_are_the_functions_fips_202_defines() {
        let long = [0xA3_u8; 200];
        for (algorithm, empty, long_message) in [
            (
                Digest::Sha3_256,
                "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a",
                "79f38adec5c20307a98ef76e8324afbfd46cfd81b22e3973c65fa1bd9de31787",
            ),
            (
                Digest::Sha3_384,
                "0c63a75b845e4f7d01107d852e4c2485c51a50aaaa94fc61995e71bbee983a2a\
                 c3713831264adb47fb6bd1e058d5f004",
                "1881de2ca7e41ef95dc4732b8f5f002b189cc1e42b74168ed1732649ce1dbcdd\
                 76197a31fd55ee989f2d7050dd473e8f",
            ),
            (
                Digest::Sha3_512,
                "a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a6\
                 15b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26",
                "e76dfad22084a8b1467fcf2ffa58361bec7628edf5f3fdc0e4805dc48caeeca8\
                 1b7c13c30adf52a3659584739a2df46be589c51ca1a4a8416df6545a1ce8ba00",
            ),
            (
                Digest::Shake256,
                "46b9dd2b0ba88d13233b3feb743eeb243fcd52ea62b81b82b50c27646ed5762f\
                 d75dc4ddd8c0f200cb05019d67b592f6fc821c49479ab48640292eacb3b7c4be",
                "cd8a920ed141aa0407a22d59288652e9d9f1a7ee0c1e7c1ca699424da84a904d\
                 2d700caae7396ece96604440577da4f3aa22aeb8857f961c4cd8e06f0ae6610b",
            ),
        ] {
            let empty = empty.replace(' ', "");
            let long_message = long_message.replace(' ', "");
            assert_eq!(
                hex(&algorithm.compute(&[])),
                empty,
                "{}, the empty message",
                algorithm.name()
            );
            assert_eq!(
                hex(&algorithm.compute(&[&long[..1], &long[1..99], &long[99..]])),
                long_message,
                "{}, 200 octets of 0xA3 in three pieces",
                algorithm.name()
            );
        }
    }

    /// ISO/TS 32001 section 5.1.4's NOTE, as a length rather than as a sentence: "[t]he requirement
    /// to use the id-shake256 OID fixes the SHAKE256 output length for the digest at 512 bits and
    /// serves to prohibit variable length SHAKE256 algorithm usage and prohibit use of SHAKE256
    /// algorithms with OIDs other than id-shake256."
    ///
    /// **The NOTE survived Errata Collection 3 and the requirement it describes did not**, so this
    /// is the one number in the round that a document here states and no document here requires;
    /// [`Digest::Shake256`] carries the choice. It is pinned by a test rather than left to the
    /// vectors because a vector cannot catch it: a prefix of a SHAKE256 stream is a valid SHAKE256
    /// output of its own length, so squeezing 32 octets still agrees with the 64-octet vector
    /// above, byte for byte.
    #[test]
    fn shake256_is_squeezed_to_the_512_bits_the_note_still_states() {
        assert_eq!(SHAKE256_OCTETS, 64);
        assert_eq!(Digest::Shake256.compute(&[b"any message at all"]).len(), 64);
        assert_eq!(
            Digest::Shake256.compute(&[]).len(),
            Digest::Sha3_512.compute(&[]).len(),
            "512 bits, the same width the standard's other longest digest has"
        );
    }

    /// The ten identifiers, decoded by this tree's own reader rather than trusted as written.
    ///
    /// `dsa.rs` applies the same discipline to its own constants and for the same reason: an
    /// identifier written as octets is a *claim about a number*, and `x509::dotted` is what checks
    /// that the octets say what the comment beside them says.
    ///
    /// ISO 32000-2 and ISO/TS 32001 print none of these digits, and Errata Collection 3 took away
    /// even the *symbol* the standard named for SHAKE256 — see [`Digest::oid`] and
    /// [`Digest::Shake256`] for the two decisions and their costs — so where a second party
    /// publishes its own transcription of the same registry, comparing the two is worth doing.
    /// That is all it is: agreement raises confidence that the registry was read correctly, which
    /// is the only thing principle 5 lets another implementation do.
    ///
    /// **Nine of the ten now carry a second reading, and six of them gained one in the
    /// six-hundred-and-eighty-ninth session**, when `const_oid`'s database came into this crate
    /// for the elliptic-curve family and turned out to hold these too at no cost (ADR 0532). The
    /// three `sha3` rows keep a *third* reading below, which is the package that computes them.
    /// RIPEMD-160 is the exception and stays one: no package in this graph publishes its
    /// identifier — `TeleTrusT`'s arc is nobody's registry here — so `1.3.36.3.2.1` stands on the
    /// transcription alone.
    #[test]
    fn the_object_identifiers_are_the_numbers_the_registry_assigns() {
        use crate::x509::dotted;
        use sha3::digest::const_oid::AssociatedOid as _;
        for (digest, expected) in [
            (Digest::Md5, "1.2.840.113549.2.5"),
            (Digest::Sha1, "1.3.14.3.2.26"),
            (Digest::Sha256, "2.16.840.1.101.3.4.2.1"),
            (Digest::Sha384, "2.16.840.1.101.3.4.2.2"),
            (Digest::Sha512, "2.16.840.1.101.3.4.2.3"),
            (Digest::Ripemd160, "1.3.36.3.2.1"),
            (Digest::Sha3_256, "2.16.840.1.101.3.4.2.8"),
            (Digest::Sha3_384, "2.16.840.1.101.3.4.2.9"),
            (Digest::Sha3_512, "2.16.840.1.101.3.4.2.10"),
            (Digest::Shake256, "2.16.840.1.101.3.4.2.12"),
        ] {
            assert_eq!(dotted(digest.oid()).as_deref(), Some(expected));
            assert_eq!(
                Digest::from_oid(digest.oid()),
                Some(digest),
                "and the pair reads the same constant in both directions"
            );
        }
        // The second reading: `const_oid`'s database, grouped by the document that assigns each.
        // MD5 is RFC 8017's `pkcs-1` neighbour, SHA-1 the OIW arc RFC 5912 records, and the six
        // NIST ones FIPS 202's and RFC 5912's. RIPEMD-160 has no row because that database has no
        // TeleTrusT identifier.
        for (digest, second) in [
            (Digest::Md5, const_oid::db::rfc5912::ID_MD_5),
            (Digest::Sha1, const_oid::db::rfc5912::ID_SHA_1),
            (Digest::Sha256, const_oid::db::rfc5912::ID_SHA_256),
            (Digest::Sha384, const_oid::db::rfc5912::ID_SHA_384),
            (Digest::Sha512, const_oid::db::rfc5912::ID_SHA_512),
            (Digest::Sha3_256, const_oid::db::fips202::ID_SHA_3_256),
            (Digest::Sha3_384, const_oid::db::fips202::ID_SHA_3_384),
            (Digest::Sha3_512, const_oid::db::fips202::ID_SHA_3_512),
            (Digest::Shake256, const_oid::db::fips202::ID_SHAKE_256),
        ] {
            assert_eq!(
                digest.oid(),
                second.as_bytes(),
                "{}: a second party's reading of the same registry",
                Digest::name(digest)
            );
        }
        // And a third for the three the `sha3` package itself publishes an identifier for.
        assert_eq!(Digest::Sha3_256.oid(), sha3::Sha3_256::OID.as_bytes());
        assert_eq!(Digest::Sha3_384.oid(), sha3::Sha3_384::OID.as_bytes());
        assert_eq!(Digest::Sha3_512.oid(), sha3::Sha3_512::OID.as_bytes());
    }

    /// Where ISO/TS 32001 puts its four, which is not everywhere.
    ///
    /// Section 5.1.4 adds them "to the Message Digest value entry for adbe.pkcs7.detached,
    /// ETSI.CAdES.detached or ETSI.RFC3161" — so `adbe.x509.rsa_sha1`, whose digest this program
    /// has to find by trying each in turn, keeps the base standard's six. A program that widened
    /// the trial set would be widening a table the standard did not.
    #[test]
    fn the_unstated_trial_set_is_the_base_standards_six() {
        assert_eq!(Digest::ALL.len(), 10);
        assert_eq!(Digest::TRIED_WHEN_UNSTATED.len(), 6);
        for digest in [
            Digest::Sha3_256,
            Digest::Sha3_384,
            Digest::Sha3_512,
            Digest::Shake256,
        ] {
            assert!(Digest::ALL.contains(&digest));
            assert!(
                !Digest::TRIED_WHEN_UNSTATED.contains(&digest),
                "{} is not in Table 260's adbe.x509.rsa_sha1 column",
                digest.name()
            );
        }
        for digest in Digest::TRIED_WHEN_UNSTATED {
            assert!(Digest::ALL.contains(&digest));
        }
    }

    /// Lower-case hexadecimal, for comparing a digest with a published vector.
    fn hex(bytes: &[u8]) -> String {
        use std::fmt::Write as _;
        bytes.iter().fold(String::new(), |mut out, byte| {
            let _ = write!(out, "{byte:02x}");
            out
        })
    }

    /// Every way a signature value can fail to be one, named rather than shrugged at.
    #[test]
    fn what_is_not_a_signed_data_says_what_it_is() {
        assert_eq!(signed_data(&[]), Err(CmsError::NotContentInfo));
        assert_eq!(
            signed_data(&[0x04, 0x01, 0x00]),
            Err(CmsError::NotContentInfo)
        );
        // A ContentInfo whose type is id-data rather than id-signedData.
        let plain = tagged(0x30, &[primitive(0x06, ID_DATA)]);
        assert_eq!(signed_data(&plain), Err(CmsError::NotSignedData));
        // A SignedData with an empty signerInfos.
        let empty = tagged(
            0x30,
            &[
                primitive(
                    0x06,
                    &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x02],
                ),
                tagged(
                    0xA0,
                    &[tagged(
                        0x30,
                        &[
                            primitive(0x02, &[0x01]),
                            tagged(0x30, &[primitive(0x06, ID_DATA)]),
                            tagged(0x31, &[]),
                        ],
                    )],
                ),
            ],
        );
        assert_eq!(signed_data(&empty), Err(CmsError::NoSigner));
    }
}
