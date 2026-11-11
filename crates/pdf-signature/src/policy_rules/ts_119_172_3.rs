//! The second machine-processable ASN.1 signature policy syntax: ETSI TS 119 172-3's
//! `SignaturePolicy`, read out of the policy document a signature is bound to and enforced over
//! that signature under ISO 32000-2 §12.8.3.4.4's "shall enforce signature policy constraints".
//!
//! TS 119 172-3 clause 4 writes its types in the Annex A module, an `EXPLICIT TAGS` module, and
//! gives none of them semantics of its own: every clause 4.n.1 sends the reader to the same clause
//! of TS 119 172-2, whose XML is another syntax for the same building blocks. So the structure
//! below is read against TS 119 172-3 and every judgement against TS 119 172-2's semantics,
//! each cited by clause; ETSI's texts are never quoted (ADR 1085).
//!
//! A document opens with clause 4.3's `Digest` — an algorithm and a value computed over the
//! binary encoding of the `PolicyComponents` beside it — and the components hold the policy's
//! identifier, its optional signing period (clause 4.13) and its rules (clauses 4.14 to 4.29).
//! Each rule may carry a recommended scope; one that names generation or augmentation alone is
//! said as such and not enforced, since TS 119 172-2 clause 4.14.1 asks users of the policy to use
//! a rule only in the scopes it names and this program is a validator.
//!
//! # What is enforced, and what is named (ADR 2580)
//!
//! Enforced: the signing period; the media types of clause 4.16 against the one data object a PDF
//! signature covers, a PDF file; the cardinality of clause 4.18 against that one object; the
//! relative position of clause 4.19, a PDF signature being embedded in the object it signs; clause
//! 4.24's mandated signing-certificate information where it asks for the signer's own certificate;
//! clause 4.25's delay between the claimed signing time and the signature timestamp; clause 4.26's
//! prohibition of signer attributes, and its constraints on sets of them as far as presence and
//! kind decide them ([`super::signer_attributes`], ADR 2787); and clause 4.29's cryptographic suites for the signature and
//! for the signing certificate, with their minimum key and hash lengths where this program
//! measures them; clause 4.24's signing-certificate trust conditions over a path built to the
//! policy's own trust points, with its `fullPath` information requirement, and clause 4.29's
//! signing-certificate key length and path-certificate usage read off that path
//! ([`super::certification_path`], ADR 2714); clause 4.25's time-stamp trust trees over the path of
//! the authority that issued the signature's time-stamp token, and clause 4.23.2's revocation checks
//! of either condition per kind of certificate ([`super::revocation_checks`], ADR 2720), from
//! material issued within clause 4.23.3's freshness where the policy states one (ADR 2775). A
//! trust point's and the authority's policy and name constraints are RFC 5280 section 6.1's
//! processing (ADR 2724). Clause 4.23.3's retention of expired revocation notices on the signing
//! certificate's revocation trust is answered by an OCSP archive cutoff (ADR 2785). Named and not
//! evaluated: the formats and levels of clause 4.20 and the levels
//! clause 4.27 ties its attribute rules to, identified by TS 119 172-2's URIs for a signature level
//! this program does not determine; augmentation, which this reader never performs; the levels of
//! assurance of clauses 4.25 and 4.28; the validity of a certified signer attribute or a signed
//! assertion, and a constraint on a signer attribute's value; and every `Other`
//! extension, whose syntax is whatever its identifier names.
//!
//! # Budgets, before the work
//!
//! The document is hostile until bound, and it is read under [`MAX_POLICY_OCTETS`] and the one
//! [`MAX_ENTRIES`] count every list entry is charged to before it is pushed, as the first syntax
//! is. **There is no distinguished-encoding pre-pass here**: TS 119 172-3 clause 4.1.3.2 admits the
//! basic encoding rules for some types, so a BER document is this syntax's own and not refused.
//! Nesting is held at [`der::MAX_DEPTH`] by [`der::Reader`] itself as each value is opened, and each
//! trust point and trust anchor of clause 4.22.5's trees is charged to the entry budget as it is
//! read.

use std::collections::BTreeSet;

use std::borrow::Cow;

use super::certification_path::{self, Path, Search, TrustPoint, TrustTrees};
use super::duration::Duration;
use super::revocation_checks::{self, Freshness};
use super::signer_attributes::{self, Accepted, Mandated, SetConstraints};
use super::{
    Budget, CertificateInfo, Finding, MAX_ENTRIES, MAX_POLICY_OCTETS, PolicyDocumentError,
    SIGNING_CERT_TRUST, SignerFacts, SignerTrust, Stated, Syntax, TIMESTAMP_TRUST, TimestampTrust,
    Verdict, signing_period, stated_commitment_type,
};
use crate::cms::{self, Digest, SignedData};
use crate::der::{self, INTEGER, OBJECT_IDENTIFIER, OCTET_STRING, SEQUENCE, Value};
use crate::signature::name;
use crate::x509;

/// `BOOLEAN`, primitive and universal.
const BOOLEAN: u8 = 0x01;
/// `ENUMERATED`, primitive and universal.
const ENUMERATED: u8 = 0x0A;
/// `UTF8String`, primitive and universal.
const UTF8_STRING: u8 = 0x0C;
/// `IA5String`, primitive and universal.
const IA5_STRING: u8 = 0x16;
/// `PrintableString`, which clause 4.23.3.2's `Duration` is.
const PRINTABLE_STRING: u8 = 0x13;
/// `GeneralizedTime`, primitive and universal.
const GENERALIZED_TIME: u8 = 0x18;

/// The media type of the one data object a PDF signature covers (RFC 8118).
const PDF_MEDIA_TYPE: &str = "application/pdf";

/// TS 119 172-2 clause 4.29.2's usage URI for algorithms that generate the digital signature.
pub const SIGNATURE_USAGE: &str = "http://uri.etsi.org/19172/v1.1.1/Usage#Signature";

/// TS 119 172-2 clause 4.29.2's usage URI for algorithms that generate the signing certificate,
/// as the schema prints it — with a `/` before the fragment that its five sibling URIs lack — and
/// as those siblings would spell it. Both are read as this one usage (ADR 2580).
pub const SIGNING_CERT_USAGE: [&str; 2] = [
    "http://uri.etsi.org/19172/v1.1.1/Usage/#SigningCert",
    "http://uri.etsi.org/19172/v1.1.1/Usage#SigningCert",
];

/// TS 119 172-2 clause 4.29.2's usage URI for algorithms that generate certificates in
/// certification paths: here the CA certificates between the signing certificate and the policy's
/// trust point, the signing certificate having a usage of its own.
pub const PATH_CERT_USAGE: &str = "http://uri.etsi.org/19172/v1.1.1/Usage#PathCert";

/// Clause 4.14.2's `SigPolicyScope`: where the policy recommends a rule be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// `generation`.
    Generation,
    /// `validation`.
    Validation,
    /// `augmentation`.
    Augmentation,
}

/// Clause 4.15.2's `MatchingIndicator`: which of a commitment rule's commitments a signature must
/// state for the rule to apply to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Matching {
    /// `all`.
    All,
    /// `none`.
    None,
    /// `atLeastOne`.
    AtLeastOne,
}

/// One bound of clause 4.18's `DTBSCardinality`, as a comparison with the number of data objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    /// `maxValueQualifier` `lessThan`.
    FewerThan,
    /// `maxValueQualifier` `lessOrEqualTo`.
    AtMost,
    /// Either qualifier's `equal`.
    Exactly,
    /// `minValueQualifier` `higherThan`.
    MoreThan,
    /// `minValueQualifier` `higherOrEqualTo`.
    AtLeast,
}

impl Comparison {
    /// Whether `count` data objects meet the bound `number`.
    #[must_use]
    pub const fn holds(self, count: i64, number: i64) -> bool {
        match self {
            Self::FewerThan => count < number,
            Self::AtMost => count <= number,
            Self::Exactly => count == number,
            Self::MoreThan => count > number,
            Self::AtLeast => count >= number,
        }
    }

    const fn words(self) -> &'static str {
        match self {
            Self::FewerThan => "fewer than",
            Self::AtMost => "at most",
            Self::Exactly => "exactly",
            Self::MoreThan => "more than",
            Self::AtLeast => "at least",
        }
    }
}

/// Clause 4.19.2's `SigDTBSRelativePositionValue`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    /// `envelopingSig`: the signature embeds the data object.
    Enveloping,
    /// `envelopedSig`: the signature is embedded within the data object.
    Enveloped,
    /// `detachedSig`: neither.
    Detached,
    /// `aSiC`: the signature is in an `ASiC` container.
    Asic,
}

impl Position {
    const fn words(self) -> &'static str {
        match self {
            Self::Enveloping => "enveloping",
            Self::Enveloped => "enveloped",
            Self::Detached => "detached",
            Self::Asic => "in an ASiC container",
        }
    }
}

/// Clause 4.18's `DTBSCardinality`: each bound it states.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cardinality {
    /// `maxDTBSNumber` and `minDTBSNumber`, each as a comparison and its number.
    pub bounds: Vec<(Comparison, i64)>,
}

/// Clause 4.20's `SigFormatsAndLevels`; an absent list admits any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatsAndLevels<'a> {
    /// `sigFormats`.
    pub formats: Option<Vec<&'a str>>,
    /// `sigLevels`.
    pub levels: Option<Vec<&'a str>>,
}

/// One of clause 4.16's `DataToBeSignedRule`s.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaTypes<'a> {
    /// `anyOfMimeType`: every signed data object has one of these.
    pub any_of: Option<Vec<&'a str>>,
    /// `noneOfMimeType`: no signed data object has one of these.
    pub none_of: Option<Vec<&'a str>>,
}

/// One of clause 4.25's `RulesForSetOfEvidences`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeEvidence<'a> {
    /// `evidenceIdentifiers`.
    pub evidences: Vec<&'a str>,
    /// `levelOfAssurance`.
    pub level_of_assurance: &'a str,
    /// `timeStampTrustCondition`'s trust trees, revocation requirement and name constraints for
    /// the time-stamping authority, where it states any of them.
    pub trust: Option<TimestampTrust<'a>>,
    /// `signatureTimestampDelay`, in seconds.
    pub delay_seconds: Option<i64>,
}

/// An identifier a policy states, as far as this program can place it.
///
/// TS 119 172-3 writes every identifier as an object identifier. TS 119 172-2 writes them as
/// URIs, an object identifier among them as RFC 3061's `urn:oid:` URN, and an algorithm
/// identifier may be a URI that names the algorithm in another registry
/// ([`super::ts_119_172_2`] places those it lists); a URI placed as neither stays a URI, which
/// no object identifier a signature states can be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Identifier<'a> {
    /// The contents octets of an object identifier.
    Oid(&'a [u8]),
    /// A URI this program does not place as an object identifier.
    Uri(&'a str),
}

impl Identifier<'_> {
    /// Whether this is the object identifier `oid`.
    #[must_use]
    pub fn is(&self, oid: &[u8]) -> bool {
        matches!(self, Self::Oid(own) if *own == oid)
    }
}

/// How an extension a policy states is named.
///
/// TS 119 172-3's `Other` carries an object identifier saying what its value is; TS 119 172-2's
/// `AnyType` (its clause 4.1.3) carries arbitrary content, named here by its first element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensionName<'a> {
    /// The extension's object identifier.
    Identifier(&'a [u8]),
    /// The `{namespace}local` name of the extension's first element, or `text` where it holds
    /// character data alone.
    Element(&'a str),
}

impl ExtensionName<'_> {
    /// How a sentence names the extension.
    #[must_use]
    pub fn sentence(&self) -> String {
        match self {
            Self::Identifier(identifier) => name(identifier),
            Self::Element(element) => (*element).to_owned(),
        }
    }
}

/// One of clause 4.29's `AlgConstraints`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlgorithmConstraint<'a> {
    /// `algID`.
    pub algorithm: Identifier<'a>,
    /// `usages`, as URIs. Empty where TS 119 172-2's `Usages` is absent, which its clause 4.29.1
    /// reads as every usage; TS 119 172-3's list holds at least one.
    pub usages: Vec<&'a str>,
    /// `minKeyLength`, in bits.
    pub min_key_length: Option<i64>,
    /// `minHashLength`, in bits.
    pub min_hash_length: Option<i64>,
    /// Each `other`, by its name.
    pub others: Vec<ExtensionName<'a>>,
    /// TS 119 172-2's `ExpirationDate`, which TS 119 172-3's type does not carry.
    pub expiration: Option<Stated>,
}

/// Clause 4.14.2's `BasicRule`, one alternative per rule type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BasicRule<'a> {
    /// `[0]` clause 4.16.
    DataToBeSigned(Vec<MediaTypes<'a>>),
    /// `[1]` clause 4.17: a cardinality, and optionally a position and formats and levels.
    Relation {
        /// `dTBSCardinality`.
        cardinality: Cardinality,
        /// `sigDTBSRelativePosition`.
        position: Option<Vec<Position>>,
        /// `sigFormatsAndLevels`.
        formats: Option<FormatsAndLevels<'a>>,
    },
    /// `[2]` clause 4.18.
    Cardinality(Cardinality),
    /// `[3]` clause 4.19.
    RelativePosition(Vec<Position>),
    /// `[4]` clause 4.20.
    FormatsAndLevels(FormatsAndLevels<'a>),
    /// `[5]` clause 4.21.
    Augmentation,
    /// `[6]` clause 4.24: its trust conditions, always stated, and `mandatedSigningCertInfo`.
    SigningCertificate {
        /// `signingCertTrustConditions`.
        trust: SignerTrust<'a>,
        /// `mandatedSigningCertInfo`.
        info: Option<CertificateInfo>,
    },
    /// `[7]` clause 4.25.
    TimeEvidences(Vec<TimeEvidence<'a>>),
    /// `[8]` clause 4.26.
    SignerAttributes {
        /// `noSignerAttributesAllowed`.
        none_allowed: bool,
        /// `constraintsOnOneSetOfAttributes`.
        sets: Vec<SetConstraints<'a>>,
    },
    /// `[9]` clause 4.27: the level each `LevelAttributesRules` is for.
    QualifyingAttributes(Vec<&'a str>),
    /// `[10]` clause 4.28.
    CreationDevice(&'a str),
    /// `[11]` clause 4.29.
    CryptoSuites(Vec<AlgorithmConstraint<'a>>),
    /// `[12]` an `Other`, by its name.
    Other(ExtensionName<'a>),
}

impl BasicRule<'_> {
    const fn words(&self) -> &'static str {
        match self {
            Self::DataToBeSigned(_) => "rule on the data to be signed",
            Self::Relation { .. } => "rule on the signature's relation to its data",
            Self::Cardinality(_) => "rule on the number of signed data objects",
            Self::RelativePosition(_) => "rule on the signature's position",
            Self::FormatsAndLevels(_) => "rule on formats and levels",
            Self::Augmentation => "augmentation rule",
            Self::SigningCertificate { .. } => "signing certificate rule",
            Self::TimeEvidences(_) => "time evidence rule",
            Self::SignerAttributes { .. } => "signer attribute rule",
            Self::QualifyingAttributes(_) => "qualifying attribute rule",
            Self::CreationDevice(_) => "signature creation device rule",
            Self::CryptoSuites(_) => "cryptographic suite rule",
            Self::Other(_) => "rule of another specification",
        }
    }
}

/// One of clause 4.15's `CommitmentRule`s.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitmentRule<'a> {
    /// Each `Commitment`'s `commitmentIdentifier`.
    pub commitments: Vec<Identifier<'a>>,
    /// `matchingIndicator`.
    pub matching: Matching,
    /// `basicRules`.
    pub rules: Vec<BasicRule<'a>>,
}

impl CommitmentRule<'_> {
    /// Whether the rule applies to a signature stating `stated` (TS 119 172-2 clause 4.15.1).
    #[must_use]
    pub fn applies(&self, stated: Option<&[u8]>) -> bool {
        let listed = |commitment: &Identifier<'_>| stated.is_some_and(|oid| commitment.is(oid));
        match self.matching {
            Matching::All => self.commitments.iter().all(listed),
            Matching::AtLeastOne => self.commitments.iter().any(listed),
            Matching::None => !self.commitments.iter().any(listed),
        }
    }
}

/// Clause 4.14.2's `SigPolicyRule`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyRule<'a> {
    /// `commitmentRules`.
    Commitments(Vec<CommitmentRule<'a>>),
    /// `basicRule`.
    Basic(BasicRule<'a>),
}

/// Clause 4.14.2's `PolicyRuleWithScope`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedRule<'a> {
    /// `rule`.
    pub rule: PolicyRule<'a>,
    /// `scope`; absent, the rule may be used in any scope.
    pub scope: Option<Scope>,
}

/// TS 119 172-3 clause 4.2's `SignaturePolicy`, as far as its constraints go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyDocument<'a> {
    /// Clause 4.3's `Digest`: its algorithm and value, re-computed by [`Self::parse`] where this
    /// crate computes the algorithm.
    pub digest: (&'a [u8], &'a [u8]),
    /// `SigPolicyDetails.policyIdentifier`.
    pub identifier: &'a [u8],
    /// `OtherDetails.signingPeriod`'s `notBefore` and `notAfter`.
    pub signing_period: Option<(Stated, Option<Stated>)>,
    /// `OtherDetails.others`, by their identifiers.
    pub other_details: Vec<&'a [u8]>,
    /// `PolicyRules`.
    pub rules: Vec<ScopedRule<'a>>,
}

/// A member TS 119 172-3 clause 4 requires is missing or of another type.
const fn malformed(member: &'static str) -> PolicyDocumentError {
    PolicyDocumentError::ComponentMalformed(member)
}

/// `value`, where it is present and carries `identifier`; else the member is named.
fn need<'a>(
    value: Option<Value<'a>>,
    identifier: u8,
    member: &'static str,
) -> Result<Value<'a>, PolicyDocumentError> {
    value
        .filter(|value| value.identifier == identifier)
        .ok_or(malformed(member))
}

/// The single value inside an `EXPLICIT` context tag.
fn explicit<'a>(
    tagged: &Value<'a>,
    member: &'static str,
) -> Result<Value<'a>, PolicyDocumentError> {
    tagged.children()?.next_value()?.ok_or(malformed(member))
}

/// The number of a constructed context-specific tag, which is what an `EXPLICIT` tag is.
fn context_number(value: &Value<'_>) -> Option<u8> {
    (value.identifier & 0xE0 == 0xA0).then_some(value.identifier & 0x1F)
}

/// A character string of the given universal type, as text.
fn text<'a>(
    value: &Value<'a>,
    tag: u8,
    member: &'static str,
) -> Result<&'a str, PolicyDocumentError> {
    if value.identifier != tag {
        return Err(malformed(member));
    }
    std::str::from_utf8(value.contents).map_err(|_| malformed(member))
}

/// `SEQUENCE OF` a character string, each entry charged to the budget; `non_empty` for a
/// `SIZE (1..MAX)` list.
fn texts<'a>(
    value: &Value<'a>,
    tag: u8,
    non_empty: bool,
    member: &'static str,
    budget: &mut Budget,
) -> Result<Vec<&'a str>, PolicyDocumentError> {
    if value.identifier != SEQUENCE {
        return Err(malformed(member));
    }
    let mut out = Vec::new();
    let mut entries = value.children()?;
    while let Some(entry) = entries.next_value()? {
        budget.take()?;
        out.push(text(&entry, tag, member)?);
    }
    if non_empty && out.is_empty() {
        return Err(malformed(member));
    }
    Ok(out)
}

/// A `BOOLEAN`; any non-zero octet is true, which the basic encoding rules clause 4.1.3.2 admits
/// allow.
fn boolean(value: &Value<'_>, member: &'static str) -> Result<bool, PolicyDocumentError> {
    match (value.identifier, value.contents) {
        (BOOLEAN, [octet]) => Ok(*octet != 0),
        _ => Err(malformed(member)),
    }
}

/// A one-octet `ENUMERATED` below `count`, which is every enumeration Annex A defines.
fn enumerated(
    value: Option<Value<'_>>,
    count: u8,
    member: &'static str,
) -> Result<u8, PolicyDocumentError> {
    match value
        .as_ref()
        .map(|value| (value.identifier, value.contents))
    {
        Some((ENUMERATED, [number])) if *number < count => Ok(*number),
        _ => Err(malformed(member)),
    }
}

/// An `INTEGER` that fits in 64 bits, sign included.
fn integer(value: Option<Value<'_>>, member: &'static str) -> Result<i64, PolicyDocumentError> {
    let value = need(value, INTEGER, member)?;
    let contents = value.contents;
    let Some(first) = contents.first() else {
        return Err(malformed(member));
    };
    if contents.len() > 8 {
        return Err(malformed(member));
    }
    let fill = if first & 0x80 == 0 { 0 } else { u64::MAX };
    let bits = contents
        .iter()
        .fold(fill, |bits, octet| bits << 8 | u64::from(*octet));
    Ok(i64::from_ne_bytes(bits.to_ne_bytes()))
}

/// The `INTEGER` inside an `EXPLICIT` context tag.
fn tagged_integer(tagged: &Value<'_>, member: &'static str) -> Result<i64, PolicyDocumentError> {
    integer(Some(explicit(tagged, member)?), member)
}

/// A clause 4.13 instant, with the spelling the document gave it.
fn instant(value: Option<Value<'_>>, member: &'static str) -> Result<Stated, PolicyDocumentError> {
    let value = need(value, GENERALIZED_TIME, member)?;
    let instant = x509::read_time(&value).ok_or(malformed(member))?;
    Ok(Stated {
        instant,
        spelled: String::from_utf8_lossy(value.contents).into_owned(),
    })
}

/// Clause 4.24.4's `SigningCertTrustConditions`: the trust trees, and the revocation trust — a
/// check of each kind of certificate, clause 4.23.3's freshness, and its retention of expired
/// revocation notices.
fn read_signer_trust<'a>(
    value: &Value<'a>,
    budget: &mut Budget,
) -> Result<SignerTrust<'a>, PolicyDocumentError> {
    let mut members = value.children()?;
    let trees = read_trust_trees(
        &need(members.next_value()?, SEQUENCE, "signerTrustTrees")?,
        budget,
    )?;
    let revocation = need(members.next_value()?, SEQUENCE, "signerRevTrust")?;
    let mut revocation = revocation.children()?;
    let mut requirement = read_revocation_requirement(&need(
        revocation.next_value()?,
        SEQUENCE,
        "certificateRevReq",
    )?)?;
    while let Some(member) = revocation.next_value()? {
        match context_number(&member) {
            Some(0) => {
                requirement.freshness = Some(read_freshness(&explicit(&member, "freshness")?)?);
            }
            Some(1) => {
                // Clause 4.23.3.2's `Duration ::= PrintableString`, the lexical form `Freshness`
                // takes.
                let name = "CertificateRevTrust.sigCertIssuedByCAKeepsExpiredRevokedCertsInfo";
                let stated = text(&explicit(&member, name)?, PRINTABLE_STRING, name)?;
                requirement.retention = Some(Duration::parse(stated).ok_or(malformed(name))?);
            }
            _ => return Err(malformed("CertificateRevTrust")),
        }
    }
    Ok(SignerTrust {
        trees,
        revocation: requirement,
    })
}

/// Clause 4.23.3.2's `Freshness` choice, each alternative a `Duration`: a `PrintableString` in the
/// XML Schema's lexical form.
fn read_freshness(value: &Value<'_>) -> Result<Freshness, PolicyDocumentError> {
    let duration = |member| {
        Duration::parse(text(&explicit(value, member)?, PRINTABLE_STRING, member)?)
            .ok_or(malformed(member))
    };
    match context_number(value) {
        Some(0) => Ok(Freshness::MaxDifference(duration(
            "Freshness.maxDifferenceRevocationAndValidation",
        )?)),
        Some(1) => Ok(Freshness::AfterSignature(duration(
            "Freshness.timeAfterSignature",
        )?)),
        _ => Err(malformed("Freshness")),
    }
}

/// Clause 4.23.2's `CertificateRevReq`: the end certificate's check and the CA certificates'.
fn read_revocation_requirement(
    value: &Value<'_>,
) -> Result<revocation_checks::Requirement, PolicyDocumentError> {
    let mut checks = value.children()?;
    let mut check = |member| {
        let number = enumerated(checks.next_value()?, 6, member)?;
        revocation_checks::Check::of_number(number).ok_or(malformed(member))
    };
    Ok(revocation_checks::Requirement {
        end_entity: check("endCertRevReq")?,
        authorities: check("caCerts")?,
        freshness: None,
        retention: None,
    })
}

/// Clause 4.22.5's `CertificateTrustTrees`: one [`TrustPoint`] per certificate trust anchor, each
/// with its trust point's constraints, and a count of the anchors given by trusted-list reference.
///
/// `acceptablePolicySet`, `nameConstraints` and `policyConstraints` are RFC 5280 section 6.1.1's
/// inputs, read as TR 102 272's are (ADR 2724). `userCertPath`'s `path` is the certification path
/// clause 4.22.5.1 mandates, which [`super::certification_path::search`] builds over in place of
/// the signature's certificates (ADR 2781); `asInSignature` true asks for the path the signature
/// carries, which is what the search builds otherwise, and `asInSignature` false names no path at
/// all, so it is kept by name and a condition stating it is never called met.
pub(super) fn read_trust_trees<'a>(
    value: &Value<'a>,
    budget: &mut Budget,
) -> Result<TrustTrees<'a>, PolicyDocumentError> {
    let mut trees = TrustTrees::default();
    let mut points = value.children()?;
    while let Some(point) = points.next_value()? {
        budget.take()?;
        let point = need(Some(point), SEQUENCE, "CertificateTrustPoint")?;
        let mut members = point.children()?;
        let anchors = need(
            members.next_value()?,
            SEQUENCE,
            "CertificateTrustPoint.trustAnchors",
        )?;
        let mut path_length = None;
        let mut unprocessed = Vec::new();
        let mut mandated_path = None;
        let mut inputs = crate::trust::PathInputs::default();
        while let Some(member) = members.next_value()? {
            match context_number(&member) {
                Some(0) => {
                    let limit = tagged_integer(&member, "pathLenConstraint")?;
                    if limit < 0 {
                        return Err(malformed("pathLenConstraint"));
                    }
                    path_length = Some(limit);
                }
                Some(number @ 1..=3) => super::trust_point_input(
                    &mut inputs,
                    &mut unprocessed,
                    number,
                    &explicit(&member, "CertificateTrustPoint")?,
                )?,
                Some(4) => {
                    let choice = explicit(&member, "userCertPath")?;
                    match context_number(&choice) {
                        Some(0) => {
                            if !boolean(&explicit(&choice, "userCertPath")?, "asInSignature")? {
                                unprocessed
                                    .push("userCertPath asInSignature false, naming no path");
                            }
                        }
                        Some(1) => {
                            let path = need(
                                Some(explicit(&choice, "userCertPath")?),
                                SEQUENCE,
                                "userCertPath.path",
                            )?;
                            let mut certificates = Vec::new();
                            let mut entries = path.children()?;
                            while let Some(certificate) = entries.next_value()? {
                                budget.take()?;
                                let certificate =
                                    need(Some(certificate), SEQUENCE, "userCertPath.path")?;
                                certificates.push(Cow::Borrowed(certificate.encoding()));
                            }
                            mandated_path = Some(certificates);
                        }
                        _ => return Err(malformed("userCertPath")),
                    }
                }
                _ => return Err(malformed("CertificateTrustPoint")),
            }
        }
        let mut entries = anchors.children()?;
        while let Some(anchor) = entries.next_value()? {
            budget.take()?;
            let anchor = need(Some(anchor), SEQUENCE, "TrustAnchor")?;
            let mut parts = anchor.children()?;
            let first = parts.next_value()?.ok_or(malformed("TrustAnchor"))?;
            match first.identifier {
                SEQUENCE => {
                    let reliable_until = parts
                        .next_value()?
                        .map(|time| instant(Some(time), "reliableUntil"))
                        .transpose()?;
                    trees.points.push(TrustPoint {
                        certificate: Cow::Borrowed(first.encoding()),
                        path_length,
                        reliable_until,
                        inputs: inputs.clone(),
                        unprocessed: unprocessed.clone(),
                        mandated_path: mandated_path.clone(),
                    });
                }
                IA5_STRING => trees.trusted_lists = trees.trusted_lists.saturating_add(1),
                _ => return Err(malformed("TrustAnchor")),
            }
        }
    }
    Ok(trees)
}

/// An `Other`'s `otherId`.
fn other_identifier<'a>(
    value: &Value<'a>,
    member: &'static str,
) -> Result<&'a [u8], PolicyDocumentError> {
    need(Some(*value), SEQUENCE, member)?
        .children()?
        .next_value()?
        .and_then(|first| first.object_identifier())
        .ok_or(malformed(member))
}

impl<'a> PolicyDocument<'a> {
    /// Reads TS 119 172-3 clause 4.2's `SignaturePolicy` out of a policy document's octets.
    ///
    /// # Errors
    ///
    /// [`PolicyDocumentError`]: the byte and entry budgets, an encoding [`der::Reader`] cannot
    /// read (its nesting past [`der::MAX_DEPTH`] included), a top level of the other syntax, a
    /// member clause 4 requires and the document lacks, and a `Digest` that disagrees with the
    /// components it is computed over.
    pub fn parse(octets: &'a [u8]) -> Result<Self, PolicyDocumentError> {
        if octets.len() > MAX_POLICY_OCTETS {
            return Err(PolicyDocumentError::TooLarge {
                octets: octets.len(),
            });
        }
        let other_syntax = || PolicyDocumentError::OtherSyntax(Syntax::Ts1191723);
        if super::ts_119_172_2::is_xml(octets) {
            return Err(other_syntax());
        }
        let mut budget = Budget(MAX_ENTRIES);
        let policy = der::Reader::new(octets)?
            .next_value()?
            .filter(|value| value.identifier == SEQUENCE)
            .ok_or_else(other_syntax)?;
        let mut members = policy.children()?;
        let digest = members
            .next_value()?
            .filter(|value| value.identifier == SEQUENCE)
            .ok_or_else(other_syntax)?;
        let mut digest = digest.children()?;
        // EN 319 122-1's `OtherHashAlgAndValue` opens with an `AlgorithmIdentifier` sequence;
        // TR 102 272's `SignaturePolicy` opens with the identifier's object identifier instead.
        let algorithm = digest
            .next_value()?
            .filter(|value| value.identifier == SEQUENCE)
            .ok_or_else(other_syntax)?
            .children()?
            .next_value()?
            .and_then(|first| first.object_identifier())
            .ok_or(malformed("Digest.hashAlgorithm"))?;
        let value = need(digest.next_value()?, OCTET_STRING, "Digest.hashValue")?.contents;
        let components = need(members.next_value()?, SEQUENCE, "PolicyComponents")?;
        if members.next_value()?.is_some() {
            return Err(malformed("SignaturePolicy"));
        }
        // Clause 4.3.2: the value is over the binary encoding of the components, which is the
        // whole value as the file wrote it. A function this crate does not compute leaves the
        // check unmade, not failed.
        if let Some(function) = Digest::from_oid(algorithm)
            && function.compute(&[components.encoding()]) != value
        {
            return Err(PolicyDocumentError::DigestDisagrees);
        }
        let mut parts = components.children()?;
        let general = need(parts.next_value()?, SEQUENCE, "GeneralDetails")?;
        let rules = need(parts.next_value()?, SEQUENCE, "PolicyRules")?;
        let (identifier, signing_period, other_details) = read_general(&general, &mut budget)?;
        let rules = read_policy_rules(&rules, &mut budget)?;
        Ok(Self {
            digest: (algorithm, value),
            identifier,
            signing_period,
            other_details,
            rules,
        })
    }
}

/// What clause 4.5's `GeneralDetails` holds that enforcement reads.
type General<'a> = (&'a [u8], Option<(Stated, Option<Stated>)>, Vec<&'a [u8]>);

/// Clause 4.5's `GeneralDetails`: the identifier, and clause 4.13's signing period and others.
fn read_general<'a>(
    general: &Value<'a>,
    budget: &mut Budget,
) -> Result<General<'a>, PolicyDocumentError> {
    let mut members = general.children()?;
    let identifier = need(members.next_value()?, SEQUENCE, "SigPolicyDetails")?
        .children()?
        .next_value()?
        .filter(|value| value.identifier == OBJECT_IDENTIFIER)
        .map(|value| value.contents)
        .ok_or(malformed("SigPolicyDetails.policyIdentifier"))?;
    let mut signing_period = None;
    let mut others = Vec::new();
    while let Some(member) = members.next_value()? {
        match context_number(&member) {
            // `authorityDetails` names the issuer and constrains nothing.
            Some(0) => {}
            Some(1) => {
                let details = need(
                    Some(explicit(&member, "otherDetails")?),
                    SEQUENCE,
                    "OtherDetails",
                )?;
                let mut fields = details.children()?;
                need(
                    fields.next_value()?,
                    GENERALIZED_TIME,
                    "OtherDetails.dateOfIssue",
                )?;
                while let Some(field) = fields.next_value()? {
                    match context_number(&field) {
                        Some(0) => {
                            let period = need(
                                Some(explicit(&field, "signingPeriod")?),
                                SEQUENCE,
                                "SigningPeriod",
                            )?;
                            let mut period = period.children()?;
                            let not_before =
                                instant(period.next_value()?, "SigningPeriod.notBefore")?;
                            let not_after = match period.next_value()? {
                                Some(value) => {
                                    Some(instant(Some(value), "SigningPeriod.notAfter")?)
                                }
                                None => None,
                            };
                            signing_period = Some((not_before, not_after));
                        }
                        Some(1) => {
                            let list = need(
                                Some(explicit(&field, "OtherDetails.others")?),
                                SEQUENCE,
                                "OtherDetails.others",
                            )?;
                            let mut entries = list.children()?;
                            while let Some(entry) = entries.next_value()? {
                                budget.take()?;
                                others.push(other_identifier(&entry, "OtherDetails.others")?);
                            }
                        }
                        _ => return Err(malformed("OtherDetails")),
                    }
                }
            }
            _ => return Err(malformed("GeneralDetails")),
        }
    }
    Ok((identifier, signing_period, others))
}

/// Clause 4.14.2's `PolicyRules`, which TS 119 172-2 clause 4.14.1 requires to state a rule.
fn read_policy_rules<'a>(
    rules: &Value<'a>,
    budget: &mut Budget,
) -> Result<Vec<ScopedRule<'a>>, PolicyDocumentError> {
    let mut out = Vec::new();
    let mut entries = rules.children()?;
    while let Some(entry) = entries.next_value()? {
        budget.take()?;
        let entry = need(Some(entry), SEQUENCE, "PolicyRuleWithScope")?;
        let mut members = entry.children()?;
        let tagged = members
            .next_value()?
            .ok_or(malformed("PolicyRuleWithScope.rule"))?;
        let scope = match members.next_value()? {
            None => None,
            Some(value) => Some(match enumerated(Some(value), 3, "SigPolicyScope")? {
                0 => Scope::Generation,
                1 => Scope::Validation,
                _ => Scope::Augmentation,
            }),
        };
        let rule = match context_number(&tagged) {
            Some(0) => PolicyRule::Commitments(read_commitment_rules(
                &explicit(&tagged, "commitmentRules")?,
                budget,
            )?),
            Some(1) => {
                PolicyRule::Basic(read_basic_rule(&explicit(&tagged, "basicRule")?, budget)?)
            }
            _ => return Err(malformed("SigPolicyRule")),
        };
        out.push(ScopedRule { rule, scope });
    }
    if out.is_empty() {
        return Err(malformed("PolicyRules"));
    }
    Ok(out)
}

/// Clause 4.15.2's `CommitmentRules`. TS 119 172-2 clause 4.15.1 requires each rule to list a
/// commitment and to state a rule.
fn read_commitment_rules<'a>(
    value: &Value<'a>,
    budget: &mut Budget,
) -> Result<Vec<CommitmentRule<'a>>, PolicyDocumentError> {
    let list = need(Some(*value), SEQUENCE, "CommitmentRules")?;
    let mut out = Vec::new();
    let mut entries = list.children()?;
    while let Some(entry) = entries.next_value()? {
        budget.take()?;
        let entry = need(Some(entry), SEQUENCE, "CommitmentRule")?;
        let mut members = entry.children()?;
        let listed = need(
            members.next_value()?,
            SEQUENCE,
            "CommitmentRule.commitments",
        )?;
        let mut commitments = Vec::new();
        let mut listed = listed.children()?;
        while let Some(commitment) = listed.next_value()? {
            budget.take()?;
            commitments.push(Identifier::Oid(
                need(Some(commitment), SEQUENCE, "Commitment")?
                    .children()?
                    .next_value()?
                    .and_then(|first| first.object_identifier())
                    .ok_or(malformed("Commitment.commitmentIdentifier"))?,
            ));
        }
        let matching = match enumerated(members.next_value()?, 3, "MatchingIndicator")? {
            0 => Matching::All,
            1 => Matching::None,
            _ => Matching::AtLeastOne,
        };
        let basic = need(members.next_value()?, SEQUENCE, "CommitmentRule.basicRules")?;
        let mut rules = Vec::new();
        let mut basic = basic.children()?;
        while let Some(rule) = basic.next_value()? {
            budget.take()?;
            rules.push(read_basic_rule(&rule, budget)?);
        }
        if commitments.is_empty() || rules.is_empty() {
            return Err(malformed("CommitmentRule"));
        }
        out.push(CommitmentRule {
            commitments,
            matching,
            rules,
        });
    }
    Ok(out)
}

/// Clause 4.14.2's `BasicRule`, a `CHOICE` of thirteen tagged alternatives.
fn read_basic_rule<'a>(
    tagged: &Value<'a>,
    budget: &mut Budget,
) -> Result<BasicRule<'a>, PolicyDocumentError> {
    let number = context_number(tagged).ok_or(malformed("BasicRule"))?;
    let inner = explicit(tagged, "BasicRule")?;
    Ok(match number {
        0 => BasicRule::DataToBeSigned(read_media_types(&inner, budget)?),
        1 => read_relation(&inner, budget)?,
        2 => BasicRule::Cardinality(read_cardinality(&inner)?),
        3 => BasicRule::RelativePosition(read_positions(&inner, budget)?),
        4 => BasicRule::FormatsAndLevels(read_formats(&inner, budget)?),
        5 => {
            need(Some(inner), SEQUENCE, "AugmentationRules")?;
            BasicRule::Augmentation
        }
        6 => {
            let rules = need(Some(inner), SEQUENCE, "SigningCertRules")?;
            let mut members = rules.children()?;
            let trust = read_signer_trust(
                &need(
                    members.next_value()?,
                    SEQUENCE,
                    "SigningCertRules.signingCertTrustConditions",
                )?,
                budget,
            )?;
            let info = match members.next_value()? {
                None => None,
                Some(value) => Some(
                    match enumerated(Some(value), 2, "MandatedSigningCertInfo")? {
                        0 => CertificateInfo::SignerOnly,
                        _ => CertificateInfo::FullPath,
                    },
                ),
            };
            BasicRule::SigningCertificate { trust, info }
        }
        7 => BasicRule::TimeEvidences(read_time_evidences(&inner, budget)?),
        8 => read_signer_attributes(&inner, budget)?,
        9 => {
            let list = need(Some(inner), SEQUENCE, "QualifyingAttributesRules")?;
            let mut levels = Vec::new();
            let mut entries = list.children()?;
            while let Some(entry) = entries.next_value()? {
                budget.take()?;
                let entry = need(Some(entry), SEQUENCE, "LevelAttributesRules")?;
                let level = entry
                    .children()?
                    .next_value()?
                    .filter(|value| context_number(value) == Some(0))
                    .ok_or(malformed("LevelAttributesRules.levelIdentifier"))?;
                levels.push(text(
                    &explicit(&level, "LevelAttributesRules.levelIdentifier")?,
                    IA5_STRING,
                    "LevelAttributesRules.levelIdentifier",
                )?);
            }
            BasicRule::QualifyingAttributes(levels)
        }
        10 => BasicRule::CreationDevice(text(&inner, IA5_STRING, "SCDLoARules")?),
        11 => BasicRule::CryptoSuites(read_crypto_suites(&inner, budget)?),
        12 => BasicRule::Other(ExtensionName::Identifier(other_identifier(
            &inner,
            "otherRules",
        )?)),
        _ => return Err(malformed("BasicRule")),
    })
}

/// Clause 4.17.2's `SigToDTBSRelationRules`.
fn read_relation<'a>(
    inner: &Value<'a>,
    budget: &mut Budget,
) -> Result<BasicRule<'a>, PolicyDocumentError> {
    let relation = need(Some(*inner), SEQUENCE, "SigToDTBSRelationRules")?;
    let mut members = relation.children()?;
    let cardinality = read_cardinality(
        &members
            .next_value()?
            .ok_or(malformed("SigToDTBSRelationRules.dTBSCardinality"))?,
    )?;
    let mut position = None;
    let mut formats = None;
    while let Some(member) = members.next_value()? {
        match context_number(&member) {
            Some(0) => {
                position = Some(read_positions(
                    &explicit(&member, "sigDTBSRelativePosition")?,
                    budget,
                )?);
            }
            Some(1) => {
                formats = Some(read_formats(
                    &explicit(&member, "sigFormatsAndLevels")?,
                    budget,
                )?);
            }
            _ => return Err(malformed("SigToDTBSRelationRules")),
        }
    }
    Ok(BasicRule::Relation {
        cardinality,
        position,
        formats,
    })
}

/// Clause 4.26.2's `SignerAttributeConstraints`: the prohibition, and the constraints on each set.
fn read_signer_attributes<'a>(
    inner: &Value<'a>,
    budget: &mut Budget,
) -> Result<BasicRule<'a>, PolicyDocumentError> {
    let constraints = need(Some(*inner), SEQUENCE, "SignerAttributeConstraints")?;
    let mut members = constraints.children()?;
    let none_allowed = boolean(
        &members.next_value()?.ok_or(malformed(
            "SignerAttributeConstraints.noSignerAttributesAllowed",
        ))?,
        "SignerAttributeConstraints.noSignerAttributesAllowed",
    )?;
    let sets = need(
        members.next_value()?,
        SEQUENCE,
        "SignerAttributeConstraints.constraintsOnOneSetOfAttributes",
    )?;
    let mut read = Vec::new();
    let mut sets = sets.children()?;
    while let Some(set) = sets.next_value()? {
        budget.take()?;
        read.push(read_attribute_set(
            &need(Some(set), SEQUENCE, "AttributeSetConstraints")?,
            budget,
        )?);
    }
    Ok(BasicRule::SignerAttributes {
        none_allowed,
        sets: read,
    })
}

/// Clause 4.26.2's `AttributeSetConstraints`: its `howCertAttribute`, its trust trees, whether its
/// revocation trust is stated, and its `AttributeConstraints`, whose value constraints TS 119
/// 172-2 clause 4.26.1 forbids without an identifier.
fn read_attribute_set<'a>(
    set: &Value<'a>,
    budget: &mut Budget,
) -> Result<SetConstraints<'a>, PolicyDocumentError> {
    let mut members = set.children()?;
    let accepted = match enumerated(
        members.next_value()?,
        4,
        "AttributeSetConstraints.howCertAttribute",
    )? {
        0 => Accepted::Claimed,
        1 => Accepted::Certified,
        2 => Accepted::SignedAssertion,
        _ => Accepted::Any,
    };
    let mut out = SetConstraints {
        accepted,
        trust_trees: None,
        revocation: false,
        mandated: Vec::new(),
    };
    let mut previous = None;
    while let Some(member) = members.next_value()? {
        let number = context_number(&member)
            .filter(|number| *number <= 2 && previous.is_none_or(|last| *number > last))
            .ok_or(malformed("AttributeSetConstraints"))?;
        previous = Some(number);
        match number {
            0 => {
                out.trust_trees = Some(read_trust_trees(
                    &need(
                        Some(explicit(&member, "attrCertificateTrustTrees")?),
                        SEQUENCE,
                        "attrCertificateTrustTrees",
                    )?,
                    budget,
                )?);
            }
            1 => out.revocation = true,
            _ => {
                let constraints = need(
                    Some(explicit(&member, "AttributeConstraints")?),
                    SEQUENCE,
                    "AttributeConstraints",
                )?;
                let mut identifier = None;
                let mut value_constraints: usize = 0;
                let mut parts = constraints.children()?;
                while let Some(part) = parts.next_value()? {
                    match context_number(&part) {
                        Some(0) if identifier.is_none() && value_constraints == 0 => {
                            identifier = Some(
                                explicit(&part, "AttributeConstraints.attributeIdMustBePresent")?
                                    .object_identifier()
                                    .ok_or(malformed(
                                        "AttributeConstraints.attributeIdMustBePresent",
                                    ))?,
                            );
                        }
                        Some(1) if value_constraints == 0 => {
                            let list = need(
                                Some(explicit(
                                    &part,
                                    "AttributeConstraints.attributeValueConstraints",
                                )?),
                                SEQUENCE,
                                "AttributeConstraints.attributeValueConstraints",
                            )?;
                            let mut entries = list.children()?;
                            while entries.next_value()?.is_some() {
                                budget.take()?;
                                value_constraints = value_constraints.saturating_add(1);
                            }
                            if value_constraints == 0 {
                                return Err(malformed(
                                    "AttributeConstraints.attributeValueConstraints",
                                ));
                            }
                        }
                        _ => return Err(malformed("AttributeConstraints")),
                    }
                }
                match identifier {
                    Some(oid) => out.mandated.push(Mandated {
                        identifier: Identifier::Oid(oid),
                        value_constraints,
                    }),
                    None if value_constraints > 0 => {
                        return Err(malformed("AttributeConstraints.attributeIdMustBePresent"));
                    }
                    None => {}
                }
            }
        }
    }
    Ok(out)
}

/// Clause 4.16.2's `DataToBeSignedRules`; TS 119 172-2 clause 4.16.2 says a rule is not empty.
fn read_media_types<'a>(
    value: &Value<'a>,
    budget: &mut Budget,
) -> Result<Vec<MediaTypes<'a>>, PolicyDocumentError> {
    let list = need(Some(*value), SEQUENCE, "DataToBeSignedRules")?;
    let mut out = Vec::new();
    let mut entries = list.children()?;
    while let Some(entry) = entries.next_value()? {
        budget.take()?;
        let entry = need(Some(entry), SEQUENCE, "DataToBeSignedRule")?;
        let mut rule = MediaTypes {
            any_of: None,
            none_of: None,
        };
        let mut members = entry.children()?;
        while let Some(member) = members.next_value()? {
            let list = explicit(&member, "DataToBeSignedRule")?;
            let list = texts(&list, UTF8_STRING, true, "DataToBeSignedRule", budget)?;
            match context_number(&member) {
                Some(0) => rule.any_of = Some(list),
                Some(1) => rule.none_of = Some(list),
                _ => return Err(malformed("DataToBeSignedRule")),
            }
        }
        if rule.any_of.is_none() && rule.none_of.is_none() {
            return Err(malformed("DataToBeSignedRule"));
        }
        out.push(rule);
    }
    Ok(out)
}

/// Clause 4.18.2's `DTBSCardinality`, which TS 119 172-2 clause 4.18.1 requires to state a bound.
fn read_cardinality(value: &Value<'_>) -> Result<Cardinality, PolicyDocumentError> {
    let cardinality = need(Some(*value), SEQUENCE, "DTBSCardinality")?;
    let mut bounds = Vec::new();
    let mut members = cardinality.children()?;
    while let Some(member) = members.next_value()? {
        let maximum = match context_number(&member) {
            Some(0) => true,
            Some(1) => false,
            _ => return Err(malformed("DTBSCardinality")),
        };
        let bound = need(
            Some(explicit(&member, "DTBSCardinality")?),
            SEQUENCE,
            "DTBSNumber",
        )?;
        let mut fields = bound.children()?;
        let number = integer(fields.next_value()?, "DTBSNumber.dTBSNumber")?;
        let comparison = match (
            maximum,
            enumerated(fields.next_value()?, 3, "DTBSNumber qualifier")?,
        ) {
            (true, 0) => Comparison::FewerThan,
            (true, 1) => Comparison::AtMost,
            (false, 0) => Comparison::MoreThan,
            (false, 1) => Comparison::AtLeast,
            _ => Comparison::Exactly,
        };
        bounds.push((comparison, number));
    }
    if bounds.is_empty() {
        return Err(malformed("DTBSCardinality"));
    }
    Ok(Cardinality { bounds })
}

/// Clause 4.19.2's `SigDTBSRelativePosition`: one to three positions, or `aSiC` alone.
fn read_positions(
    value: &Value<'_>,
    budget: &mut Budget,
) -> Result<Vec<Position>, PolicyDocumentError> {
    let list = need(Some(*value), SEQUENCE, "SigDTBSRelativePosition")?;
    let mut out = Vec::new();
    let mut entries = list.children()?;
    while let Some(entry) = entries.next_value()? {
        budget.take()?;
        out.push(
            match enumerated(Some(entry), 4, "SigDTBSRelativePositionValue")? {
                0 => Position::Enveloping,
                1 => Position::Enveloped,
                2 => Position::Detached,
                _ => Position::Asic,
            },
        );
    }
    let asic_with_another = out.contains(&Position::Asic) && out.len() > 1;
    if out.is_empty() || out.len() > 3 || asic_with_another {
        return Err(malformed("SigDTBSRelativePosition"));
    }
    Ok(out)
}

/// Clause 4.20.2's `SigFormatsAndLevels`, which is not empty.
fn read_formats<'a>(
    value: &Value<'a>,
    budget: &mut Budget,
) -> Result<FormatsAndLevels<'a>, PolicyDocumentError> {
    let formats_and_levels = need(Some(*value), SEQUENCE, "SigFormatsAndLevels")?;
    let mut out = FormatsAndLevels {
        formats: None,
        levels: None,
    };
    let mut members = formats_and_levels.children()?;
    while let Some(member) = members.next_value()? {
        let list = texts(
            &explicit(&member, "SigFormatsAndLevels")?,
            IA5_STRING,
            true,
            "SigFormatsAndLevels",
            budget,
        )?;
        match context_number(&member) {
            Some(0) => out.formats = Some(list),
            Some(1) => out.levels = Some(list),
            _ => return Err(malformed("SigFormatsAndLevels")),
        }
    }
    if out.formats.is_none() && out.levels.is_none() {
        return Err(malformed("SigFormatsAndLevels"));
    }
    Ok(out)
}

/// Clause 4.25.2's `TimeEvidencesRules`.
fn read_time_evidences<'a>(
    value: &Value<'a>,
    budget: &mut Budget,
) -> Result<Vec<TimeEvidence<'a>>, PolicyDocumentError> {
    let list = need(Some(*value), SEQUENCE, "TimeEvidencesRules")?;
    let mut out = Vec::new();
    let mut entries = list.children()?;
    while let Some(entry) = entries.next_value()? {
        budget.take()?;
        let entry = need(Some(entry), SEQUENCE, "RulesForSetOfEvidences")?;
        let mut members = entry.children()?;
        let evidences = texts(
            &members
                .next_value()?
                .ok_or(malformed("RulesForSetOfEvidences.evidenceIdentifiers"))?,
            IA5_STRING,
            false,
            "RulesForSetOfEvidences.evidenceIdentifiers",
            budget,
        )?;
        let level_of_assurance = text(
            &members
                .next_value()?
                .ok_or(malformed("RulesForSetOfEvidences.levelOfAssurance"))?,
            IA5_STRING,
            "RulesForSetOfEvidences.levelOfAssurance",
        )?;
        let mut trust: Option<TimestampTrust<'a>> = None;
        let mut delay_seconds = None;
        if let Some(condition) = members.next_value()? {
            let condition = need(Some(condition), SEQUENCE, "TimestampTrustCondition")?;
            let mut fields = condition.children()?;
            while let Some(field) = fields.next_value()? {
                let stated = || TimestampTrust {
                    trees: None,
                    revocation: None,
                    names: None,
                    caution_seconds: None,
                    delay_seconds: None,
                    unprocessed: Vec::new(),
                };
                match context_number(&field) {
                    Some(0) => {
                        trust.get_or_insert_with(stated).trees = Some(read_trust_trees(
                            &need(
                                Some(explicit(&field, "ttsCertificateTrustTrees")?),
                                SEQUENCE,
                                "ttsCertificateTrustTrees",
                            )?,
                            budget,
                        )?);
                    }
                    Some(1) => {
                        trust.get_or_insert_with(stated).revocation =
                            Some(read_revocation_requirement(&need(
                                Some(explicit(&field, "ttsRevReq")?),
                                SEQUENCE,
                                "ttsRevReq",
                            )?)?);
                    }
                    Some(2) => {
                        let condition = trust.get_or_insert_with(stated);
                        condition.names = super::authority_names(
                            &explicit(&field, "ttsNameConstraints")?,
                            &mut condition.unprocessed,
                        )?;
                    }
                    Some(3) => {
                        delay_seconds = Some(read_delta_time(&explicit(
                            &field,
                            "signatureTimestampDelay",
                        )?)?);
                    }
                    _ => return Err(malformed("TimestampTrustCondition")),
                }
            }
        }
        out.push(TimeEvidence {
            evidences,
            level_of_assurance,
            trust,
            delay_seconds,
        });
    }
    Ok(out)
}

/// Clause 4.25.2's `DeltaTime`, in seconds; a sum past 64 bits is not a delay. TR 102 272 clause
/// 6.8 defines the same type, and its reader is this one.
pub(super) fn read_delta_time(value: &Value<'_>) -> Result<i64, PolicyDocumentError> {
    let delta = need(Some(*value), SEQUENCE, "DeltaTime")?;
    let mut fields = delta.children()?;
    let mut total: i64 = 0;
    for (member, unit) in [
        ("DeltaTime.deltaSeconds", 1_i64),
        ("DeltaTime.deltaMinutes", 60),
        ("DeltaTime.deltaHours", 3_600),
        ("DeltaTime.deltaDays", 86_400),
    ] {
        total = integer(fields.next_value()?, member)?
            .checked_mul(unit)
            .and_then(|seconds| total.checked_add(seconds))
            .ok_or(malformed(member))?;
    }
    Ok(total)
}

/// Clause 4.29.2's `CryptoSuitesRules`.
fn read_crypto_suites<'a>(
    value: &Value<'a>,
    budget: &mut Budget,
) -> Result<Vec<AlgorithmConstraint<'a>>, PolicyDocumentError> {
    let list = need(Some(*value), SEQUENCE, "CryptoSuitesRules")?;
    let mut out = Vec::new();
    let mut entries = list.children()?;
    while let Some(entry) = entries.next_value()? {
        budget.take()?;
        let entry = need(Some(entry), SEQUENCE, "AlgConstraints")?;
        let mut members = entry.children()?;
        let algorithm = need(
            members.next_value()?,
            OBJECT_IDENTIFIER,
            "AlgConstraints.algID",
        )?
        .contents;
        let usages = texts(
            &members
                .next_value()?
                .ok_or(malformed("AlgConstraints.usages"))?,
            IA5_STRING,
            true,
            "AlgConstraints.usages",
            budget,
        )?;
        let mut constraint = AlgorithmConstraint {
            algorithm: Identifier::Oid(algorithm),
            usages,
            min_key_length: None,
            min_hash_length: None,
            others: Vec::new(),
            expiration: None,
        };
        while let Some(member) = members.next_value()? {
            match context_number(&member) {
                Some(0) => {
                    constraint.min_key_length =
                        Some(tagged_integer(&member, "AlgConstraints.minKeyLength")?);
                }
                Some(1) => {
                    constraint.min_hash_length =
                        Some(tagged_integer(&member, "AlgConstraints.minHashLength")?);
                }
                _ if member.identifier == SEQUENCE => {
                    constraint
                        .others
                        .push(ExtensionName::Identifier(other_identifier(
                            &member,
                            "AlgConstraints.other",
                        )?));
                }
                _ => return Err(malformed("AlgConstraints")),
            }
        }
        out.push(constraint);
    }
    Ok(out)
}

/// What one usage of clause 4.29 is said as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Usage {
    /// [`SIGNATURE_USAGE`].
    Signature,
    /// [`SIGNING_CERT_USAGE`].
    SigningCertificate,
    /// [`PATH_CERT_USAGE`].
    PathCertificate,
}

/// A constraint of TS 119 172-3 that TR 102 272's vocabulary has no counterpart for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Constraint {
    /// Clause 4.16's `anyOfMimeType`.
    AnyOfMediaType(Vec<String>),
    /// Clause 4.16's `noneOfMimeType`.
    NoneOfMediaType(Vec<String>),
    /// One bound of clause 4.18's cardinality.
    DataObjectCount {
        /// The comparison.
        comparison: Comparison,
        /// Its number.
        number: i64,
    },
    /// Clause 4.19's relative position.
    RelativePosition(Vec<Position>),
    /// Clause 4.20's formats and levels.
    FormatsAndLevels {
        /// The format URIs, where stated.
        formats: Option<Vec<String>>,
        /// The level URIs, where stated.
        levels: Option<Vec<String>>,
    },
    /// Clause 4.21's augmentation rules.
    Augmentation,
    /// Clause 4.25's level of assurance on a set of time evidences.
    TimeEvidence {
        /// The evidences, by their identifiers.
        evidences: Vec<String>,
        /// The level of assurance.
        level_of_assurance: String,
    },
    /// Clause 4.25's delay between the claimed signing time and the signature timestamp.
    TimestampDelay {
        /// The limit, in seconds.
        limit_seconds: i64,
        /// The claimed signing time, as the signer spelled it.
        claimed: Option<String>,
        /// The time the signature timestamp states, as it spells it.
        stamped: Option<String>,
        /// The elapsed seconds, where both are known.
        elapsed_seconds: Option<i64>,
    },
    /// Clause 4.26's `noSignerAttributesAllowed`.
    NoSignerAttributes,
    /// Clause 4.27's attribute rules for one signature level.
    QualifyingAttributes(String),
    /// Clause 4.28's level of assurance of the signature creation device.
    CreationDevice(String),
    /// One algorithm a signature or its signing certificate used, against clause 4.29.
    SuiteAlgorithm {
        /// The usage.
        usage: Usage,
        /// The algorithm, dotted, where this program read it.
        algorithm: Option<String>,
        /// Whether a constraint lists it for the usage.
        listed: bool,
        /// That constraint's `minKeyLength`.
        min_key_length: Option<i64>,
        /// The key length this program measured against it.
        key_bits: Option<usize>,
        /// That constraint's `minHashLength`.
        min_hash_length: Option<i64>,
        /// The hash length this program measured against it.
        hash_bits: Option<usize>,
    },
    /// A clause 4.29 usage this program does not evaluate, by its URI.
    SuiteUsage(String),
    /// A clause 4.29 algorithm named by a URI this program does not place as an object
    /// identifier, by that URI.
    UnplacedAlgorithm(String),
    /// TS 119 172-2 clause 4.29.1's expiration date of an algorithm, which the clause compares
    /// with no instant it names.
    AlgorithmExpiration {
        /// The algorithm, dotted or as its URI.
        algorithm: String,
        /// The date as written.
        expiration: String,
    },
    /// A rule whose recommended scope leaves out validation.
    OutsideValidation {
        /// The rule, in words.
        rule: &'static str,
        /// Its scope.
        scope: Scope,
    },
}

/// §12.8.3.4.4's enforcement over a parsed TS 119 172-3 document: every constraint judged against
/// one signature.
#[must_use]
pub fn findings(
    document: &PolicyDocument<'_>,
    cms: &SignedData<'_>,
    facts: &SignerFacts<'_>,
) -> Vec<Finding> {
    let mut out = Vec::new();
    if let Some((not_before, not_after)) = &document.signing_period {
        out.push(signing_period(facts, not_before, not_after.as_ref()));
    }
    judge_rules(&document.rules, cms, facts, &mut out);
    for identifier in &document.other_details {
        out.push(Finding {
            verdict: Verdict::NotEvaluated,
            constraint: super::Constraint::Extension(name(identifier)),
        });
    }
    out
}

/// The findings of a policy's rules, in either syntax that writes TS 119 172-2's building blocks:
/// each rule judged in the scope it is recommended for, and a commitment rule's rules where its
/// matching indicator selects the signature.
pub(super) fn judge_rules(
    rules: &[ScopedRule<'_>],
    cms: &SignedData<'_>,
    facts: &SignerFacts<'_>,
    out: &mut Vec<Finding>,
) {
    let stated = stated_commitment_type(cms);
    // The signing-certificate rule that applies is searched once, and every path-dependent
    // constraint — its own full-path requirement and clause 4.29's usages along the path — reads
    // that one path (ADR 2714).
    let search = rules
        .iter()
        .filter(|scoped| !matches!(scoped.scope, Some(Scope::Generation | Scope::Augmentation)))
        .flat_map(|scoped| match &scoped.rule {
            PolicyRule::Basic(rule) => vec![rule],
            PolicyRule::Commitments(rules) => rules
                .iter()
                .filter(|rule| rule.applies(stated))
                .flat_map(|rule| &rule.rules)
                .collect(),
        })
        .find_map(|rule| match rule {
            BasicRule::SigningCertificate { trust, .. } => Some(trust),
            _ => None,
        })
        .map(|trust| certification_path::search(&trust.trees, cms, facts));
    let search = search.as_ref();
    let authorities = certification_path::Authorities::default();
    let paths = Paths {
        signer: search,
        authorities: &authorities,
    };
    for scoped in rules {
        if let Some(scope @ (Scope::Generation | Scope::Augmentation)) = scoped.scope {
            let rule = match &scoped.rule {
                PolicyRule::Commitments(_) => "commitment rules",
                PolicyRule::Basic(rule) => rule.words(),
            };
            out.push(component(
                Verdict::NotEvaluated,
                Constraint::OutsideValidation { rule, scope },
            ));
            continue;
        }
        match &scoped.rule {
            PolicyRule::Basic(rule) => judge(rule, cms, facts, paths, out),
            PolicyRule::Commitments(rules) => {
                for rule in rules.iter().filter(|rule| rule.applies(stated)) {
                    for basic in &rule.rules {
                        judge(basic, cms, facts, paths, out);
                    }
                }
            }
        }
    }
}

const fn component(verdict: Verdict, constraint: Constraint) -> Finding {
    Finding {
        verdict,
        constraint: super::Constraint::PolicyComponent(constraint),
    }
}

const fn verdict(held: bool) -> Verdict {
    if held { Verdict::Met } else { Verdict::Broken }
}

fn owned(list: &[&str]) -> Vec<String> {
    list.iter().map(|entry| (*entry).to_owned()).collect()
}

/// The signer's path, searched once, and the bounded searches for time-stamping authorities' paths.
#[derive(Debug, Clone, Copy)]
struct Paths<'s> {
    signer: Option<&'s Search>,
    authorities: &'s certification_path::Authorities,
}

/// One rule's findings.
fn judge(
    rule: &BasicRule<'_>,
    cms: &SignedData<'_>,
    facts: &SignerFacts<'_>,
    paths: Paths<'_>,
    out: &mut Vec<Finding>,
) {
    let search = paths.signer;
    let path = search.and_then(|search| search.path.as_ref());
    match rule {
        BasicRule::DataToBeSigned(rules) => {
            // §12.8.1: the byte range a signature digests is a range of the PDF file, so the one
            // data object signed is a PDF document; media types compare without case (RFC 2045
            // section 5.1).
            let is_pdf = |entry: &&str| entry.eq_ignore_ascii_case(PDF_MEDIA_TYPE);
            for rule in rules {
                if let Some(any_of) = &rule.any_of {
                    out.push(component(
                        verdict(any_of.iter().any(is_pdf)),
                        Constraint::AnyOfMediaType(owned(any_of)),
                    ));
                }
                if let Some(none_of) = &rule.none_of {
                    out.push(component(
                        verdict(!none_of.iter().any(is_pdf)),
                        Constraint::NoneOfMediaType(owned(none_of)),
                    ));
                }
            }
        }
        BasicRule::Relation {
            cardinality,
            position,
            formats,
        } => {
            judge_cardinality(cardinality, out);
            if let Some(position) = position {
                judge_position(position, out);
            }
            if let Some(formats) = formats {
                judge_formats(formats, out);
            }
        }
        BasicRule::Cardinality(cardinality) => judge_cardinality(cardinality, out),
        BasicRule::RelativePosition(position) => judge_position(position, out),
        BasicRule::FormatsAndLevels(formats) => judge_formats(formats, out),
        BasicRule::Augmentation => {
            out.push(component(Verdict::NotEvaluated, Constraint::Augmentation));
        }
        BasicRule::SigningCertificate { trust, info } => {
            judge_signing_certificate(trust, *info, cms, facts, search, out);
        }
        BasicRule::TimeEvidences(rules) => {
            for rule in rules {
                judge_time_evidence(rule, cms, facts, paths.authorities, out);
            }
        }
        BasicRule::SignerAttributes { none_allowed, sets } => {
            if *none_allowed {
                let carried = [cms::ID_AA_ETS_SIGNER_ATTR_V2, cms::ID_AA_ETS_SIGNER_ATTR]
                    .iter()
                    .any(|oid| cms.has_signed_attribute(oid));
                out.push(component(verdict(!carried), Constraint::NoSignerAttributes));
            }
            let context = signer_attributes::Context {
                facts,
                signer_path: paths.signer,
                signer_trees: None,
            };
            for set in sets {
                signer_attributes::judge_set(set, cms, context, out);
            }
        }
        BasicRule::QualifyingAttributes(levels) => {
            for level in levels {
                out.push(component(
                    Verdict::NotEvaluated,
                    Constraint::QualifyingAttributes((*level).to_owned()),
                ));
            }
        }
        BasicRule::CreationDevice(level) => {
            out.push(component(
                Verdict::NotEvaluated,
                Constraint::CreationDevice((*level).to_owned()),
            ));
        }
        BasicRule::CryptoSuites(list) => judge_suites(list, cms, facts, path, out),
        BasicRule::Other(other) => out.push(Finding {
            verdict: Verdict::NotEvaluated,
            constraint: super::Constraint::Extension(other.sentence()),
        }),
    }
}

/// Clause 4.24 over the path the applicable signing-certificate rule's search found: its trust
/// conditions, its revocation trust, and `mandatedSigningCertInfo`.
fn judge_signing_certificate(
    trust: &SignerTrust<'_>,
    info: Option<CertificateInfo>,
    cms: &SignedData<'_>,
    facts: &SignerFacts<'_>,
    search: Option<&Search>,
    out: &mut Vec<Finding>,
) {
    // The rule that applies was searched; a second signing-certificate rule in another
    // place is said with that one's search, the trees being clause 4.24's one condition.
    let searched = search
        .cloned()
        .unwrap_or_else(|| certification_path::search(&trust.trees, cms, facts));
    super::trust_path_findings(
        SIGNING_CERT_TRUST,
        Some(&searched),
        Some(&trust.revocation),
        facts,
        timing(facts),
        out,
    );
    if let Some(info) = info {
        out.push(Finding {
            verdict: match info {
                // A path is built over the certificates the signature carries alone.
                CertificateInfo::FullPath if searched.path.is_some() => Verdict::Met,
                CertificateInfo::FullPath => Verdict::NotEvaluated,
                _ => verdict(facts.carries_signer_certificate),
            },
            constraint: super::Constraint::CertificateInfo(info),
        });
    }
}

/// Clause 4.18 against the one data object a PDF signature covers (§12.8.1).
fn judge_cardinality(cardinality: &Cardinality, out: &mut Vec<Finding>) {
    for (comparison, number) in &cardinality.bounds {
        out.push(component(
            verdict(comparison.holds(1, *number)),
            Constraint::DataObjectCount {
                comparison: *comparison,
                number: *number,
            },
        ));
    }
}

/// Clause 4.19: a PDF signature is embedded within the data object it signs, which ETSI EN 319
/// 102-1 clause 3.1 calls enveloped; whether the file arrived in an `ASiC` container the file
/// cannot say.
fn judge_position(position: &[Position], out: &mut Vec<Finding>) {
    let held = if position.contains(&Position::Asic) {
        Verdict::NotEvaluated
    } else {
        verdict(position.iter().all(|value| *value == Position::Enveloped))
    };
    out.push(component(
        held,
        Constraint::RelativePosition(position.to_vec()),
    ));
}

fn judge_formats(formats: &FormatsAndLevels<'_>, out: &mut Vec<Finding>) {
    out.push(component(
        Verdict::NotEvaluated,
        Constraint::FormatsAndLevels {
            formats: formats.formats.as_deref().map(owned),
            levels: formats.levels.as_deref().map(owned),
        },
    ));
}

/// Clause 4.25, where TS 119 172-2 clause 4.25.1 measures the delay in validation from the claimed
/// signing time, where the signature states one, to the signature timestamp's generation.
fn judge_time_evidence(
    rule: &TimeEvidence<'_>,
    cms: &SignedData<'_>,
    facts: &SignerFacts<'_>,
    authorities: &certification_path::Authorities,
    out: &mut Vec<Finding>,
) {
    out.push(component(
        Verdict::NotEvaluated,
        Constraint::TimeEvidence {
            evidences: owned(&rule.evidences),
            level_of_assurance: rule.level_of_assurance.to_owned(),
        },
    ));
    // Clause 4.25.1 item 3: the trust trees, revocation trust and name constraints for the
    // authority that issued the signature's time-stamp token, over that authority's own path
    // (ADR 2720). Unlike TR 102 272 clause 6.8, no rule applies the signing-certificate trees
    // where the condition states none.
    if let Some(trust) = &rule.trust {
        let search = trust
            .trees
            .as_ref()
            .map(|trees| authorities.search(trees, cms, facts));
        super::trust_path_findings(
            TIMESTAMP_TRUST,
            search.as_ref(),
            trust.revocation.as_ref(),
            facts,
            timing(facts),
            out,
        );
        super::authority_name_finding(TIMESTAMP_TRUST, search.as_ref(), trust.names.as_ref(), out);
        for member in &trust.unprocessed {
            out.push(Finding {
                verdict: Verdict::NotEvaluated,
                constraint: super::Constraint::ConditionMember {
                    condition: TIMESTAMP_TRUST,
                    member,
                },
            });
        }
    }
    if let Some(limit_seconds) = rule.delay_seconds {
        out.push(delay_finding(limit_seconds, cms, facts));
    }
}

/// What places a revocation trust's freshness on the time line: the best-signature time. TS 119
/// 172-2 and -3 state no caution period, which is TR 102 272 clause 6.8's alone.
fn timing(facts: &SignerFacts<'_>) -> revocation_checks::Timing {
    revocation_checks::Timing {
        stamped: super::stamped_at(facts),
        caution_seconds: None,
    }
}

/// The delay between the claimed signing time and the signature timestamp's generation, against
/// `limit_seconds`: clause 4.25's `signatureTimestampDelay` here, and TR 102 272 clause 6.8's,
/// which states the same limit in the same `DeltaTime`. ETSI EN 319 102-1 clause 5.5.4 step 5
/// leaves a signature claiming no signing time indeterminate, so it is not evaluated.
pub(super) fn delay_finding(
    limit_seconds: i64,
    cms: &SignedData<'_>,
    facts: &SignerFacts<'_>,
) -> Finding {
    let claimed = cms
        .attribute(cms::ID_SIGNING_TIME)
        .filter(|attribute| attribute.signed)
        .and_then(|attribute| attribute.first)
        .and_then(|value| {
            Some((
                x509::read_time(&value)?,
                String::from_utf8_lossy(value.contents).into_owned(),
            ))
        });
    let stamped = facts
        .signing_time
        .as_ref()
        .filter(|at| at.source == super::TimeSource::SignatureTimestamp);
    let elapsed_seconds = match (&claimed, stamped) {
        (Some((claimed, _)), Some(stamped)) => stamped
            .instant
            .unix_seconds()
            .checked_sub(claimed.unix_seconds()),
        _ => None,
    };
    component(
        elapsed_seconds.map_or(Verdict::NotEvaluated, |elapsed| {
            verdict(elapsed <= limit_seconds)
        }),
        Constraint::TimestampDelay {
            limit_seconds,
            claimed: claimed.map(|(_, spelled)| spelled),
            stamped: stamped.map(|at| at.spelled.clone()),
            elapsed_seconds,
        },
    )
}

/// Clause 4.29. TS 119 172-2 clause 4.29.1 states the usages in which an algorithm is allowed, so
/// for a usage the list names, an algorithm used there must be listed for it; a usage no entry
/// names is not constrained (ADR 2580).
fn judge_suites(
    list: &[AlgorithmConstraint<'_>],
    cms: &SignedData<'_>,
    facts: &SignerFacts<'_>,
    path: Option<&Path>,
    out: &mut Vec<Finding>,
) {
    let undecided =
        |finding: Finding, listed: bool, usage: &[&str]| undecided(list, finding, listed, usage);
    if list.iter().any(|entry| names(entry, &[SIGNATURE_USAGE])) {
        for (used, is_signature) in [
            (cms.digest_algorithm, false),
            (cms.signature_algorithm, true),
        ] {
            let entry = list
                .iter()
                .find(|entry| entry.algorithm.is(used) && names(entry, &[SIGNATURE_USAGE]));
            out.push(undecided(
                suite_finding(
                    Usage::Signature,
                    Some(used),
                    entry,
                    is_signature.then_some(facts.key_bits).flatten(),
                    is_signature,
                ),
                entry.is_some(),
                &[SIGNATURE_USAGE],
            ));
        }
    }
    if list.iter().any(|entry| names(entry, &SIGNING_CERT_USAGE)) {
        let used = facts.certificate_signature_algorithm.as_deref();
        let entry = used.and_then(|used| {
            list.iter()
                .find(|entry| entry.algorithm.is(used) && names(entry, &SIGNING_CERT_USAGE))
        });
        // The key that made the signing certificate is its issuer's, known from the path.
        let issuer_key_bits = path
            .and_then(Path::end_entity)
            .and_then(|issued| issued.issuer_key_bits);
        out.push(undecided(
            suite_finding(
                Usage::SigningCertificate,
                used,
                entry,
                issuer_key_bits,
                true,
            ),
            entry.is_some(),
            &SIGNING_CERT_USAGE,
        ));
    }
    judge_path_suites(list, path, out);
    for entry in list {
        if let Identifier::Uri(uri) = entry.algorithm {
            out.push(component(
                Verdict::NotEvaluated,
                Constraint::UnplacedAlgorithm(uri.to_owned()),
            ));
        }
        if let Some(expiration) = &entry.expiration {
            out.push(component(
                Verdict::NotEvaluated,
                Constraint::AlgorithmExpiration {
                    algorithm: match entry.algorithm {
                        Identifier::Oid(oid) => name(oid),
                        Identifier::Uri(uri) => uri.to_owned(),
                    },
                    expiration: expiration.spelled.clone(),
                },
            ));
        }
    }
    let mut others = BTreeSet::new();
    for entry in list {
        for usage in &entry.usages {
            if *usage != SIGNATURE_USAGE
                && *usage != PATH_CERT_USAGE
                && !SIGNING_CERT_USAGE.contains(usage)
            {
                others.insert(*usage);
            }
        }
    }
    for usage in others {
        out.push(component(
            Verdict::NotEvaluated,
            Constraint::SuiteUsage(usage.to_owned()),
        ));
    }
    for other in list.iter().flat_map(|entry| &entry.others) {
        out.push(Finding {
            verdict: Verdict::NotEvaluated,
            constraint: super::Constraint::Extension(other.sentence()),
        });
    }
}

/// Whether a clause 4.29 entry names one of `usage`'s URIs; an entry naming none names every usage
/// (TS 119 172-2 clause 4.29.1).
fn names(entry: &AlgorithmConstraint<'_>, usage: &[&str]) -> bool {
    entry.usages.is_empty() || entry.usages.iter().any(|named| usage.contains(named))
}

/// A finding for an algorithm no placed entry lists, made not evaluated rather than broken where an
/// entry whose algorithm stays a URI names the usage: that URI may be the algorithm used.
fn undecided(
    list: &[AlgorithmConstraint<'_>],
    mut finding: Finding,
    listed: bool,
    usage: &[&str],
) -> Finding {
    let unplaced = list
        .iter()
        .any(|entry| matches!(entry.algorithm, Identifier::Uri(_)) && names(entry, usage));
    if !listed && finding.verdict == Verdict::Broken && unplaced {
        finding.verdict = Verdict::NotEvaluated;
    }
    finding
}

/// Clause 4.29's path-certificate usage over the CA certificates between the signing certificate and
/// the policy's trust point, each with the key that issued it (ADR 2714).
fn judge_path_suites(
    list: &[AlgorithmConstraint<'_>],
    path: Option<&Path>,
    out: &mut Vec<Finding>,
) {
    if list.iter().any(|entry| names(entry, &[PATH_CERT_USAGE])) {
        let authorities = path.map(Path::authorities).unwrap_or_default();
        if authorities.is_empty() {
            out.push(component(
                if path.is_some() {
                    Verdict::Met
                } else {
                    Verdict::NotEvaluated
                },
                Constraint::SuiteAlgorithm {
                    usage: Usage::PathCertificate,
                    algorithm: None,
                    listed: false,
                    min_key_length: None,
                    key_bits: None,
                    min_hash_length: None,
                    hash_bits: None,
                },
            ));
        }
        for issued in &authorities {
            let used = issued.algorithm.as_slice();
            let entry = list
                .iter()
                .find(|entry| entry.algorithm.is(used) && names(entry, &[PATH_CERT_USAGE]));
            out.push(undecided(
                list,
                suite_finding(
                    Usage::PathCertificate,
                    Some(used),
                    entry,
                    issued.issuer_key_bits,
                    true,
                ),
                entry.is_some(),
                &[PATH_CERT_USAGE],
            ));
        }
    }
}

/// One used algorithm against its entry. A key length is measured only where `key_bits` is the
/// key that made the signature or the certificate — the signer's, or the issuer's along the path
/// — and a hash length only where the algorithm is a hash function this crate computes; a minimum
/// stated where neither is measured is not evaluated rather than met.
fn suite_finding(
    usage: Usage,
    used: Option<&[u8]>,
    entry: Option<&AlgorithmConstraint<'_>>,
    key_bits: Option<usize>,
    is_signature: bool,
) -> Finding {
    let hash_bits = used
        .and_then(Digest::from_oid)
        .map(|function| function.compute(&[]).len().saturating_mul(8));
    let at_least = |minimum: i64, measured: Option<usize>| {
        measured.map(|bits| i64::try_from(bits).is_ok_and(|bits| bits >= minimum))
    };
    let held = match (used, entry) {
        (None, _) => Verdict::NotEvaluated,
        (Some(_), None) => Verdict::Broken,
        (Some(_), Some(entry)) => {
            let checks = [
                entry
                    .min_key_length
                    .map(|minimum| at_least(minimum, key_bits.filter(|_| is_signature))),
                entry
                    .min_hash_length
                    .map(|minimum| at_least(minimum, hash_bits)),
            ];
            if checks.contains(&Some(Some(false))) {
                Verdict::Broken
            } else if checks.contains(&Some(None)) {
                Verdict::NotEvaluated
            } else {
                Verdict::Met
            }
        }
    };
    component(
        held,
        Constraint::SuiteAlgorithm {
            usage,
            algorithm: used.map(name),
            listed: entry.is_some(),
            min_key_length: entry.and_then(|entry| entry.min_key_length),
            key_bits: key_bits.filter(|_| is_signature),
            min_hash_length: entry.and_then(|entry| entry.min_hash_length),
            hash_bits,
        },
    )
}

fn list(entries: &[String]) -> String {
    entries.join(", ")
}

impl Constraint {
    /// The sentence a reader is shown, with the verdict in its place.
    pub(super) fn sentence(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        verdict: &str,
    ) -> std::fmt::Result {
        match self {
            Self::AnyOfMediaType(types) => write!(
                f,
                "the policy's requirement that every signed data object be one of {} is {verdict}: a PDF signature signs one PDF document, {PDF_MEDIA_TYPE}",
                list(types)
            ),
            Self::NoneOfMediaType(types) => write!(
                f,
                "the policy's requirement that no signed data object be one of {} is {verdict}: a PDF signature signs one PDF document, {PDF_MEDIA_TYPE}",
                list(types)
            ),
            Self::DataObjectCount { comparison, number } => write!(
                f,
                "the policy's requirement that a signature sign {} {number} data object(s) is {verdict}: a PDF signature signs one",
                comparison.words()
            ),
            Self::RelativePosition(positions) => {
                let positions: Vec<&str> =
                    positions.iter().map(|position| position.words()).collect();
                write!(
                    f,
                    "the policy's requirement that the signature be {} is {verdict}: a PDF signature is enveloped, embedded within the document it signs, and whether the file came in an ASiC container it does not say",
                    positions.join(" and ")
                )
            }
            Self::FormatsAndLevels { formats, levels } => {
                formats_sentence(f, verdict, formats.as_deref(), levels.as_deref())
            }
            Self::Augmentation => write!(
                f,
                "the policy's augmentation rules are {verdict}: this program never augments a signature"
            ),
            Self::TimeEvidence {
                evidences,
                level_of_assurance,
            } => write!(
                f,
                "the policy's level of assurance {level_of_assurance} on time evidences {} is {verdict}: this program does not assess levels of assurance",
                list(evidences)
            ),
            Self::TimestampDelay {
                limit_seconds,
                claimed,
                stamped,
                elapsed_seconds,
            } => delay_sentence(
                f,
                verdict,
                *limit_seconds,
                claimed.as_deref(),
                stamped.as_deref(),
                *elapsed_seconds,
            ),
            Self::NoSignerAttributes => write!(
                f,
                "the policy's prohibition of signer attributes is {verdict}"
            ),
            Self::QualifyingAttributes(level) => write!(
                f,
                "the policy's attribute rules for signatures of level {level} are {verdict}: this program does not determine a signature's level"
            ),
            Self::CreationDevice(level) => write!(
                f,
                "the policy's level of assurance {level} for the signature creation device is {verdict}: nothing in a signature says what device made it"
            ),
            Self::SuiteAlgorithm {
                usage,
                algorithm,
                listed,
                min_key_length,
                key_bits,
                min_hash_length,
                hash_bits,
            } => suite_sentence(
                f,
                verdict,
                *usage,
                algorithm.as_deref(),
                *listed,
                (*min_key_length, *key_bits),
                (*min_hash_length, *hash_bits),
            ),
            Self::SuiteUsage(_) | Self::UnplacedAlgorithm(_) | Self::AlgorithmExpiration { .. } => {
                self.suite_entry_sentence(f, verdict)
            }
            Self::OutsideValidation { rule, scope } => write!(
                f,
                "the policy's {rule} is {verdict}: the policy recommends it for {} and not for validation",
                match scope {
                    Scope::Generation => "signature generation",
                    Scope::Validation => "validation",
                    Scope::Augmentation => "signature augmentation",
                }
            ),
        }
    }
}

impl Constraint {
    /// The sentences of the suite constraints this program names and does not evaluate.
    fn suite_entry_sentence(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        verdict: &str,
    ) -> std::fmt::Result {
        match self {
            Self::SuiteUsage(usage) => write!(
                f,
                "the policy's cryptographic suite rules for usage {usage} are {verdict}: they constrain algorithms this program does not check against the policy"
            ),
            Self::UnplacedAlgorithm(uri) => write!(
                f,
                "the policy's cryptographic suite entry for algorithm {uri} is {verdict}: the URI names no algorithm this program can match to an object identifier"
            ),
            Self::AlgorithmExpiration {
                algorithm,
                expiration,
            } => write!(
                f,
                "the policy's expiration date {expiration} for algorithm {algorithm} is {verdict}: ETSI TS 119 172-2 clause 4.29.1 does not say which instant it is compared with"
            ),
            _ => Ok(()),
        }
    }
}

/// [`Constraint::FormatsAndLevels`]'s sentence.
fn formats_sentence(
    f: &mut std::fmt::Formatter<'_>,
    verdict: &str,
    formats: Option<&[String]>,
    levels: Option<&[String]>,
) -> std::fmt::Result {
    write!(f, "the policy's admitted signature")?;
    if let Some(formats) = formats {
        write!(f, " formats {}", list(formats))?;
    }
    if let Some(levels) = levels {
        write!(f, " levels {}", list(levels))?;
    }
    write!(
        f,
        " are {verdict}: this program does not determine which format and level a signature is"
    )
}

/// [`Constraint::TimestampDelay`]'s sentence.
fn delay_sentence(
    f: &mut std::fmt::Formatter<'_>,
    verdict: &str,
    limit_seconds: i64,
    claimed: Option<&str>,
    stamped: Option<&str>,
    elapsed_seconds: Option<i64>,
) -> std::fmt::Result {
    write!(
        f,
        "the policy's limit of {limit_seconds} s between the claimed signing time and the signature timestamp is {verdict}"
    )?;
    match (claimed, stamped, elapsed_seconds) {
        (Some(claimed), Some(stamped), Some(elapsed)) => write!(
            f,
            ": the signer claims {claimed} and the timestamp states {stamped}, {elapsed} s later"
        ),
        (None, _, _) => write!(f, ": the signature claims no signing time"),
        _ => write!(f, ": no signature timestamp covers the signature"),
    }
}

/// [`Constraint::SuiteAlgorithm`]'s sentence.
fn suite_sentence(
    f: &mut std::fmt::Formatter<'_>,
    verdict: &str,
    usage: Usage,
    algorithm: Option<&str>,
    listed: bool,
    key: (Option<i64>, Option<usize>),
    hash: (Option<i64>, Option<usize>),
) -> std::fmt::Result {
    let usage = match usage {
        Usage::Signature => "the signature",
        Usage::SigningCertificate => "the signing certificate",
        Usage::PathCertificate => "a CA certificate on the path to the policy's trust point",
    };
    let Some(algorithm) = algorithm else {
        if usage.starts_with("a CA") {
            return write!(
                f,
                "the policy's cryptographic suite rules for certificates in certification paths are {verdict}: {}",
                if verdict == "met" {
                    "no CA certificate stands between the signing certificate and the policy's trust point"
                } else {
                    "no path to the policy's own trust points validated"
                }
            );
        }
        return write!(
            f,
            "the policy's cryptographic suite rules for {usage} are {verdict}: this program read no signing certificate"
        );
    };
    if !listed {
        return write!(
            f,
            "the policy's cryptographic suite rules for {usage} are {verdict}: they do not admit {algorithm}"
        );
    }
    write!(
        f,
        "the policy's cryptographic suite rules admit {algorithm} for {usage}"
    )?;
    for (what, (minimum, measured)) in [("key", key), ("hash", hash)] {
        if let Some(minimum) = minimum {
            match measured {
                Some(bits) => write!(
                    f,
                    ", with a {what} of at least {minimum} bits against {bits}"
                )?,
                None => write!(
                    f,
                    ", with a {what} of at least {minimum} bits this program does not measure here"
                )?,
            }
        }
    }
    write!(f, ", and that constraint is {verdict}")
}

#[cfg(test)]
mod tests;
