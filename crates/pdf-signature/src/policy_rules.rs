//! The constraints of a machine-processable signature policy, read out of the policy document a
//! signature is bound to and enforced over that signature.
//!
//! ISO 32000-2 §12.8.3.4.4 ends the explicit-policy profile's paragraph on a validator's duty:
//!
//! > During validation conforming signature handlers should not enforce seed values constraints,
//! > if present, but shall enforce signature policy constraints.
//!
//! The constraints live inside the policy document, and [`crate::policy`] already answers whether
//! the document a file carries is the one the signer committed to. This module is the next step
//! for one syntax: the ASN.1 signature policy of ETSI TR 102 272 clause 6 and its Annex A (the
//! syntax RFC 3125 also publishes), whose signature validation policy states a signing period,
//! the attributes a signer must provide, the certificates a signer must reference and carry, the
//! commitment types the policy recognises and the algorithms it admits. ETSI's texts are cited by
//! clause and never quoted: their notice permits no reproduction (ADR 1085).
//!
//! # What is enforced, and what is named
//!
//! Each constraint becomes one [`Finding`], whose [`Verdict`] is met, broken, or not evaluated —
//! and a constraint this program cannot evaluate is said to be one, never counted as met (ADR
//! 2560). Enforced: the signing period of clause 6.2 against the signature timestamp's instant,
//! the mandated signed and unsigned attributes of clause 6.5.1, its `externalSignedData`, its
//! certificate reference and information requirements where the signer's own certificate decides
//! them, the commitment-type selection of clause 6.4, and clause 6.10's signer algorithm
//! constraints with their minimum key length. Named and not evaluated: the three trust conditions
//! of clauses 6.7 to 6.9, whose trust points are a host's anchors (ADR 1076) rather than the
//! policy's; the certificate-issuer algorithm constraints of clause 6.10; the requirements that
//! reach the full certification path; the verifier rules of clause 6.5.2, which ask a verifier to
//! *add* attributes and so are an augmentation this reader never performs; and every extension of
//! clause 6.11, whose syntax is whatever its own identifier names.
//!
//! # Budgets, before the work
//!
//! A policy document arrives in clause 5.2.10's unsigned store, so it is hostile input until the
//! binding says otherwise, and it is read under three bounds that are checked before anything is
//! allocated: its length ([`MAX_POLICY_OCTETS`]), its nesting ([`der::MAX_DEPTH`], enforced by the
//! DER pre-pass [`der::is_canonical`], which walks the whole document before a single list is
//! built), and the total number of list entries it may state ([`MAX_ENTRIES`]). Clause 6.1 says
//! the syntax is encoded under the distinguished rules, so the same pre-pass refuses a document
//! that is not DER by name.

use crate::cms::{self, Digest, SignedData};
use crate::der::{self, DerError, NotCanonical, OBJECT_IDENTIFIER, OCTET_STRING, SEQUENCE, Value};
use crate::policy::{Binding, PolicyHash, SignaturePolicy, Store};
use crate::signature::name;
use crate::x509::{self, Instant, PublicKey};

/// The longest policy document this module reads, in octets.
///
/// A signature validation policy is a few kilobytes: its lists are object identifiers and its
/// names are short strings. A quarter of a megabyte is two orders of magnitude of room, and it
/// exists so that a store stuffed with an enormous value is refused before the DER walk rather
/// than walked.
pub const MAX_POLICY_OCTETS: usize = 256 * 1024;

/// The most list entries one policy document may state, counted across every list it holds.
///
/// Each `SEQUENCE OF` clause 6 defines — attributes, commitment rules and the types they select,
/// algorithm constraints, extensions — is read into a vector, so this one count is what bounds
/// the allocation a document can cause: every entry is counted against it before it is pushed.
pub const MAX_ENTRIES: usize = 4096;

/// Clause 6.7's `signingCertTrustCondition`, as a sentence names it.
pub const SIGNING_CERT_TRUST: &str = "signing certificate trust condition";
/// Clause 6.8's `timeStampTrustCondition`, as a sentence names it.
pub const TIMESTAMP_TRUST: &str = "time-stamp trust condition";
/// Clause 6.9's `attributeTrustCondition`, as a sentence names it.
pub const ATTRIBUTE_TRUST: &str = "attribute trust condition";

/// One rule of clause 6.3's placement: its name, whether a rule set states it, and whether it is
/// one of the three that must be stated somewhere.
type Placement = (&'static str, fn(&Rules<'_>) -> bool, bool);

/// `BOOLEAN`, primitive and universal.
const BOOLEAN: u8 = 0x01;
/// `ENUMERATED`, primitive and universal.
const ENUMERATED: u8 = 0x0A;
/// `GeneralizedTime`, primitive and universal.
const GENERALIZED_TIME: u8 = 0x18;

/// Why a policy document's constraints were not read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PolicyDocumentError {
    /// The document is longer than [`MAX_POLICY_OCTETS`].
    #[error(
        "the policy document is {octets} octets long, past the {MAX_POLICY_OCTETS} this program reads"
    )]
    TooLarge {
        /// Its length.
        octets: usize,
    },
    /// The encoding itself stopped the reader, its nesting past [`der::MAX_DEPTH`] included.
    #[error("the policy document's encoding cannot be read: {0}")]
    Encoding(#[from] DerError),
    /// The document is BER where ETSI TR 102 272 clause 6.1 says the syntax is DER.
    #[error(
        "the policy document is not in the distinguished encoding its syntax is written in: {0:?}"
    )]
    NotDistinguished(NotCanonical),
    /// The document is not ETSI TR 102 272's `SignaturePolicy` at its top level.
    ///
    /// ETSI TS 119 172-3's ASN.1 policy, whose top level opens with a digest rather than an
    /// algorithm identifier, is the other machine-processable ASN.1 syntax, and it is told apart
    /// here by that first member rather than read as this one.
    #[error(
        "the policy document is not written in ETSI TR 102 272's ASN.1 signature policy syntax"
    )]
    OtherSyntax,
    /// A member clause 6 requires is missing or of another type; the member is named.
    #[error(
        "the policy document's {0} is missing or is not the type ETSI TR 102 272 clause 6 gives it"
    )]
    Malformed(&'static str),
    /// The document states more list entries than [`MAX_ENTRIES`].
    #[error("the policy document states more than {MAX_ENTRIES} list entries")]
    TooManyEntries,
    /// The policy's own hash, re-computed as clause 6.1 requires, is not the one it states.
    #[error(
        "the policy document's own hash, re-computed over its policy information, is not the value it states"
    )]
    OwnHashDisagrees,
    /// A rule stated in the common rules is stated again in a commitment rule, which clause 6.3
    /// forbids; the rule is named.
    #[error(
        "the policy document states its {0} both among its common rules and in a commitment rule"
    )]
    RuleInBothPlaces(&'static str),
    /// A rule clause 6.3 requires in the common rules or else in every commitment rule is in
    /// neither; the rule is named.
    #[error(
        "the policy document's {0} is neither among its common rules nor in every commitment rule"
    )]
    RuleInNeitherPlace(&'static str),
    /// One commitment type is selected by two commitment rules, which clause 6.4 forbids.
    #[error("the policy document selects commitment type {0} in more than one commitment rule")]
    CommitmentTypeInTwoRules(String),
}

/// Clause 6.5.1's `CertRefReq`: which certificates the signer's signing-certificate attribute
/// must reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertificateReference {
    /// `signerOnly`, the default: the signer's own certificate.
    SignerOnly,
    /// `fullPath`: every certificate of the path up to a trust point.
    FullPath,
}

/// Clause 6.5.1's `CertInfoReq`: which certificates the `SignedData` must carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertificateInfo {
    /// `none`, the default.
    None,
    /// `signerOnly`: the signer's own certificate.
    SignerOnly,
    /// `fullPath`: every certificate of the path up to a trust point.
    FullPath,
}

/// Clause 6.5.1's `SignerRules`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignerRules<'a> {
    /// `externalSignedData`: `Some(true)` where the signed data must be outside the CMS object,
    /// `Some(false)` where it must be inside, `None` where either is allowed.
    pub external_signed_data: Option<bool>,
    /// `mandatedSignedAttr`, as object identifiers.
    pub mandated_signed: Vec<&'a [u8]>,
    /// `mandatedUnsignedAttr`, as object identifiers.
    pub mandated_unsigned: Vec<&'a [u8]>,
    /// `mandatedCertificateRef`.
    pub certificate_reference: CertificateReference,
    /// `mandatedCertificateInfo`.
    pub certificate_info: CertificateInfo,
}

/// One of clause 6.10's `AlgAndLength`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgorithmAndLength<'a> {
    /// `algID`.
    pub algorithm: &'a [u8],
    /// `minKeyLength`, in bits.
    pub min_key_length: Option<u32>,
}

/// One rule set: clause 6.3's `CommonRules`, or the rules a clause 6.4 `CommitmentRule` adds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rules<'a> {
    /// `signerAndVeriferRules`'s signer half.
    pub signer: Option<SignerRules<'a>>,
    /// `signerAndVeriferRules`'s verifier half: the unsigned attributes a verifier is to add.
    pub verifier_adds: Option<Vec<&'a [u8]>>,
    /// Which of clauses 6.7 to 6.9's trust conditions are stated, by [`SIGNING_CERT_TRUST`],
    /// [`TIMESTAMP_TRUST`] and [`ATTRIBUTE_TRUST`].
    pub trust_conditions: Vec<&'static str>,
    /// Whether `algorithmConstraintSet` is stated at all.
    pub algorithm_constraints: bool,
    /// `signerAlgorithmConstraints`, where stated.
    pub signer_algorithms: Option<Vec<AlgorithmAndLength<'a>>>,
    /// Which of the four certificate-issuer constraint lists are stated, by their member names.
    pub certificate_algorithms: Vec<&'static str>,
}

/// One clause 6.4 `CommitmentRule`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitmentRule<'a> {
    /// `selCommitmentTypes`: `None` for the `empty` choice, else the commitment type's identifier.
    pub selected: Vec<Option<&'a [u8]>>,
    /// The rules it states.
    pub rules: Rules<'a>,
}

/// A clause 6.2 instant, with the spelling the document gave it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stated {
    /// The instant.
    pub instant: Instant,
    /// The `GeneralizedTime` as written, which is what a sentence shows.
    pub spelled: String,
}

/// ETSI TR 102 272 clause 6's `SignaturePolicy`, as far as its constraints go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyDocument<'a> {
    /// `signPolicyIdentifier`.
    pub identifier: &'a [u8],
    /// `signPolicyHashAlg` and `signPolicyHash`, where the hash is stated: the policy's own hash
    /// over its policy information, which clause 6.1 says is re-calculated and checked.
    pub own_hash: Option<(&'a [u8], &'a [u8])>,
    /// `signingPeriod.notBefore`.
    pub not_before: Stated,
    /// `signingPeriod.notAfter`.
    pub not_after: Option<Stated>,
    /// `commonRules`.
    pub common: Rules<'a>,
    /// `commitmentRules`.
    pub commitment_rules: Vec<CommitmentRule<'a>>,
    /// Every clause 6.11 `SignPolExtn`'s identifier, wherever in the document it is stated.
    pub extensions: Vec<&'a [u8]>,
}

/// The one counter every list entry is charged to.
struct Budget(usize);

impl Budget {
    fn take(&mut self) -> Result<(), PolicyDocumentError> {
        self.0 = self
            .0
            .checked_sub(1)
            .ok_or(PolicyDocumentError::TooManyEntries)?;
        Ok(())
    }
}

impl<'a> PolicyDocument<'a> {
    /// Reads ETSI TR 102 272 clause 6's `SignaturePolicy` out of a policy document's octets.
    ///
    /// # Errors
    ///
    /// [`PolicyDocumentError`]: the three budgets the module documentation names, an encoding
    /// that is not DER, a top level of another syntax, a member clause 6 requires and the document
    /// lacks, the policy's own hash disagreeing with its information, and clause 6.3's and 6.4's
    /// structural rules.
    pub fn parse(octets: &'a [u8]) -> Result<Self, PolicyDocumentError> {
        if octets.len() > MAX_POLICY_OCTETS {
            return Err(PolicyDocumentError::TooLarge {
                octets: octets.len(),
            });
        }
        // The whole document is walked once, depth-bounded, before anything below allocates.
        if let Some(departure) = der::is_canonical(octets)? {
            return Err(PolicyDocumentError::NotDistinguished(departure));
        }
        let mut budget = Budget(MAX_ENTRIES);
        let mut top = der::Reader::new(octets)?;
        let policy = expect(top.next_value()?, SEQUENCE, "SignaturePolicy")?;
        let mut members = policy.children()?;
        let algorithm = members
            .next_value()?
            .filter(|value| value.identifier == SEQUENCE)
            .ok_or(PolicyDocumentError::OtherSyntax)?;
        // An `AlgorithmIdentifier` opens with its object identifier; TS 119 172-3's `Digest`
        // opens with an `AlgorithmIdentifier` instead, which is how the two are told apart.
        let algorithm = algorithm
            .children()?
            .next_value()?
            .and_then(|first| first.object_identifier())
            .ok_or(PolicyDocumentError::OtherSyntax)?;
        let info = expect(members.next_value()?, SEQUENCE, "signPolicyInfo")?;
        let own_hash = match members.next_value()? {
            Some(value) if value.identifier == OCTET_STRING => Some((algorithm, value.contents)),
            Some(_) => return Err(PolicyDocumentError::Malformed("signPolicyHash")),
            None => None,
        };
        if let Some((algorithm, value)) = own_hash {
            // Clause 6.1: the hash is over the policy information without its outer type and
            // length. A function this crate does not compute leaves the check unmade, not failed.
            if let Some(digest) = Digest::from_oid(algorithm)
                && digest.compute(&[info.contents]) != value
            {
                return Err(PolicyDocumentError::OwnHashDisagrees);
            }
        }
        let mut extensions = Vec::new();
        let mut fields = info.children()?;
        let identifier = expect(
            fields.next_value()?,
            OBJECT_IDENTIFIER,
            "signPolicyIdentifier",
        )?
        .contents;
        expect(fields.next_value()?, GENERALIZED_TIME, "dateOfIssue")?;
        expect(fields.next_value()?, SEQUENCE, "policyIssuerName")?;
        fields
            .next_value()?
            .ok_or(PolicyDocumentError::Malformed("fieldOfApplication"))?;
        let validation = expect(fields.next_value()?, SEQUENCE, "signatureValidationPolicy")?;
        if let Some(value) = fields.next_value()? {
            read_extensions(&value, &mut extensions, &mut budget)?;
        }
        let mut validation = validation.children()?;
        let period = expect(validation.next_value()?, SEQUENCE, "signingPeriod")?;
        let mut period = period.children()?;
        let not_before = stated(period.next_value()?, "signingPeriod.notBefore")?;
        let not_after = match period.next_value()? {
            Some(value) => Some(stated(Some(value), "signingPeriod.notAfter")?),
            None => None,
        };
        let common = expect(validation.next_value()?, SEQUENCE, "commonRules")?;
        let common = read_rules(&mut common.children()?, &mut extensions, &mut budget)?;
        let rules = expect(validation.next_value()?, SEQUENCE, "commitmentRules")?;
        let mut commitment_rules = Vec::new();
        let mut entries = rules.children()?;
        while let Some(rule) = entries.next_value()? {
            budget.take()?;
            commitment_rules.push(read_commitment_rule(&rule, &mut extensions, &mut budget)?);
        }
        if let Some(value) = validation.next_value()? {
            read_extensions(&value, &mut extensions, &mut budget)?;
        }
        let document = Self {
            identifier,
            own_hash,
            not_before,
            not_after,
            common,
            commitment_rules,
            extensions,
        };
        document.check_structure()?;
        Ok(document)
    }

    /// Clause 6.3's placement rule and clause 6.4's uniqueness rule.
    fn check_structure(&self) -> Result<(), PolicyDocumentError> {
        let placements: [Placement; 5] = [
            ("signer and verifier rules", |r| r.signer.is_some(), true),
            (
                SIGNING_CERT_TRUST,
                |r| r.trust_conditions.contains(&SIGNING_CERT_TRUST),
                true,
            ),
            (
                TIMESTAMP_TRUST,
                |r| r.trust_conditions.contains(&TIMESTAMP_TRUST),
                true,
            ),
            (
                ATTRIBUTE_TRUST,
                |r| r.trust_conditions.contains(&ATTRIBUTE_TRUST),
                false,
            ),
            (
                "algorithm constraint set",
                |r| r.algorithm_constraints,
                false,
            ),
        ];
        for (rule, stated, required) in placements {
            let in_common = stated(&self.common);
            if in_common && self.commitment_rules.iter().any(|c| stated(&c.rules)) {
                return Err(PolicyDocumentError::RuleInBothPlaces(rule));
            }
            if required && !in_common && !self.commitment_rules.iter().all(|c| stated(&c.rules)) {
                return Err(PolicyDocumentError::RuleInNeitherPlace(rule));
            }
        }
        let mut seen: Vec<Option<&[u8]>> = Vec::new();
        for selected in self.commitment_rules.iter().flat_map(|c| &c.selected) {
            if seen.contains(selected) {
                return Err(PolicyDocumentError::CommitmentTypeInTwoRules(
                    selected.map_or_else(|| "empty".to_owned(), name),
                ));
            }
            seen.push(*selected);
        }
        Ok(())
    }

    /// The commitment rule that applies to a signature stating `stated`, under clause 6.4.
    ///
    /// `Err(())` where the policy states commitment rules and none fits: a signature stating a
    /// type no rule selects, or stating none where no rule selects `empty`.
    fn commitment_rule(&self, stated: Option<&[u8]>) -> Result<Option<&CommitmentRule<'a>>, ()> {
        if self.commitment_rules.is_empty() {
            return Ok(None);
        }
        self.commitment_rules
            .iter()
            .find(|rule| rule.selected.contains(&stated))
            .map(Some)
            .ok_or(())
    }
}

/// `value`, where it is present and carries `identifier`; else the member is named.
fn expect<'a>(
    value: Option<Value<'a>>,
    identifier: u8,
    member: &'static str,
) -> Result<Value<'a>, PolicyDocumentError> {
    value
        .filter(|value| value.identifier == identifier)
        .ok_or(PolicyDocumentError::Malformed(member))
}

/// A `GeneralizedTime` this reader can place on a line.
fn stated(value: Option<Value<'_>>, member: &'static str) -> Result<Stated, PolicyDocumentError> {
    let value = expect(value, GENERALIZED_TIME, member)?;
    let instant = x509::read_time(&value).ok_or(PolicyDocumentError::Malformed(member))?;
    Ok(Stated {
        instant,
        spelled: String::from_utf8_lossy(value.contents).into_owned(),
    })
}

/// The single value inside an `EXPLICIT` context tag — the module is `EXPLICIT TAGS`.
fn explicit<'a>(
    tagged: &Value<'a>,
    member: &'static str,
) -> Result<Value<'a>, PolicyDocumentError> {
    tagged
        .children()?
        .next_value()?
        .ok_or(PolicyDocumentError::Malformed(member))
}

/// `SEQUENCE OF OBJECT IDENTIFIER`, each entry charged to the budget.
fn read_identifiers<'a>(
    value: &Value<'a>,
    member: &'static str,
    budget: &mut Budget,
) -> Result<Vec<&'a [u8]>, PolicyDocumentError> {
    if value.identifier != SEQUENCE {
        return Err(PolicyDocumentError::Malformed(member));
    }
    let mut out = Vec::new();
    let mut entries = value.children()?;
    while let Some(entry) = entries.next_value()? {
        budget.take()?;
        out.push(
            entry
                .object_identifier()
                .ok_or(PolicyDocumentError::Malformed(member))?,
        );
    }
    Ok(out)
}

/// Clause 6.11's `SignPolExtensions`: the identifiers are kept, the values are not read.
fn read_extensions<'a>(
    value: &Value<'a>,
    out: &mut Vec<&'a [u8]>,
    budget: &mut Budget,
) -> Result<(), PolicyDocumentError> {
    if value.identifier != SEQUENCE {
        return Err(PolicyDocumentError::Malformed("signPolExtensions"));
    }
    let mut entries = value.children()?;
    while let Some(entry) = entries.next_value()? {
        budget.take()?;
        let entry = expect(Some(entry), SEQUENCE, "signPolExtn")?;
        let identifier = entry
            .children()?
            .next_value()?
            .and_then(|first| first.object_identifier())
            .ok_or(PolicyDocumentError::Malformed("signPolExtn.extnID"))?;
        out.push(identifier);
    }
    Ok(())
}

/// The `[0]` to `[5]` members clause 6.3's `CommonRules` and clause 6.4's `CommitmentRule` share.
fn read_rules<'a>(
    members: &mut der::Reader<'a>,
    extensions: &mut Vec<&'a [u8]>,
    budget: &mut Budget,
) -> Result<Rules<'a>, PolicyDocumentError> {
    let mut rules = Rules::default();
    while let Some(member) = members.next_value()? {
        let tag = (member.class() == der::Class::ContextSpecific && member.is_constructed())
            .then(|| member.tag_number())
            .ok_or(PolicyDocumentError::Malformed("a rule's context tag"))?;
        match tag {
            0 => {
                let both = expect(
                    Some(explicit(&member, "signerAndVeriferRules")?),
                    SEQUENCE,
                    "signerAndVeriferRules",
                )?;
                let mut both = both.children()?;
                let signer = expect(both.next_value()?, SEQUENCE, "signerRules")?;
                rules.signer = Some(read_signer_rules(&signer, extensions, budget)?);
                let verifier = expect(both.next_value()?, SEQUENCE, "verifierRules")?;
                let mut verifier = verifier.children()?;
                let adds = verifier
                    .next_value()?
                    .ok_or(PolicyDocumentError::Malformed(
                        "verifierRules.mandatedUnsignedAttr",
                    ))?;
                rules.verifier_adds = Some(read_identifiers(
                    &adds,
                    "verifierRules.mandatedUnsignedAttr",
                    budget,
                )?);
                if let Some(value) = verifier.next_value()? {
                    read_extensions(&value, extensions, budget)?;
                }
            }
            1 => rules.trust_conditions.push(SIGNING_CERT_TRUST),
            2 => rules.trust_conditions.push(TIMESTAMP_TRUST),
            3 => rules.trust_conditions.push(ATTRIBUTE_TRUST),
            4 => {
                rules.algorithm_constraints = true;
                let set = expect(
                    Some(explicit(&member, "algorithmConstraintSet")?),
                    SEQUENCE,
                    "algorithmConstraintSet",
                )?;
                let mut lists = set.children()?;
                while let Some(list) = lists.next_value()? {
                    match (list.class(), list.tag_number()) {
                        (der::Class::ContextSpecific, 0) => {
                            rules.signer_algorithms = Some(read_algorithms(
                                &explicit(&list, "signerAlgorithmConstraints")?,
                                extensions,
                                budget,
                            )?);
                        }
                        (der::Class::ContextSpecific, number @ 1..=4) => {
                            let member = match number {
                                1 => "eeCertAlgorithmConstraints",
                                2 => "caCertAlgorithmConstraints",
                                3 => "aaCertAlgorithmConstraints",
                                _ => "tsaCertAlgorithmConstraints",
                            };
                            read_algorithms(&explicit(&list, member)?, extensions, budget)?;
                            rules.certificate_algorithms.push(member);
                        }
                        _ => return Err(PolicyDocumentError::Malformed("algorithmConstraintSet")),
                    }
                }
            }
            5 => read_extensions(&explicit(&member, "signPolExtensions")?, extensions, budget)?,
            _ => return Err(PolicyDocumentError::Malformed("a rule's context tag")),
        }
    }
    Ok(rules)
}

/// Clause 6.5.1's `SignerRules`.
fn read_signer_rules<'a>(
    value: &Value<'a>,
    extensions: &mut Vec<&'a [u8]>,
    budget: &mut Budget,
) -> Result<SignerRules<'a>, PolicyDocumentError> {
    let mut members = value.children()?;
    let mut next = members.next_value()?;
    let external_signed_data = match next {
        Some(value) if value.identifier == BOOLEAN => {
            next = members.next_value()?;
            match value.contents {
                [0x00] => Some(false),
                [_] => Some(true),
                _ => {
                    return Err(PolicyDocumentError::Malformed(
                        "signerRules.externalSignedData",
                    ));
                }
            }
        }
        _ => None,
    };
    let signed = next.ok_or(PolicyDocumentError::Malformed(
        "signerRules.mandatedSignedAttr",
    ))?;
    let mandated_signed = read_identifiers(&signed, "signerRules.mandatedSignedAttr", budget)?;
    let unsigned = members.next_value()?.ok_or(PolicyDocumentError::Malformed(
        "signerRules.mandatedUnsignedAttr",
    ))?;
    let mandated_unsigned =
        read_identifiers(&unsigned, "signerRules.mandatedUnsignedAttr", budget)?;
    let mut rules = SignerRules {
        external_signed_data,
        mandated_signed,
        mandated_unsigned,
        certificate_reference: CertificateReference::SignerOnly,
        certificate_info: CertificateInfo::None,
    };
    while let Some(member) = members.next_value()? {
        match (member.class(), member.tag_number()) {
            (der::Class::ContextSpecific, 0) => {
                rules.certificate_reference =
                    match enumerated(&explicit(&member, "mandatedCertificateRef")?)? {
                        1 => CertificateReference::SignerOnly,
                        2 => CertificateReference::FullPath,
                        _ => {
                            return Err(PolicyDocumentError::Malformed(
                                "signerRules.mandatedCertificateRef",
                            ));
                        }
                    };
            }
            (der::Class::ContextSpecific, 1) => {
                rules.certificate_info =
                    match enumerated(&explicit(&member, "mandatedCertificateInfo")?)? {
                        0 => CertificateInfo::None,
                        1 => CertificateInfo::SignerOnly,
                        2 => CertificateInfo::FullPath,
                        _ => {
                            return Err(PolicyDocumentError::Malformed(
                                "signerRules.mandatedCertificateInfo",
                            ));
                        }
                    };
            }
            (der::Class::ContextSpecific, 2) => {
                read_extensions(&explicit(&member, "signPolExtensions")?, extensions, budget)?;
            }
            _ => return Err(PolicyDocumentError::Malformed("signerRules")),
        }
    }
    Ok(rules)
}

/// A one-octet `ENUMERATED`, which is every enumeration clause 6 defines.
fn enumerated(value: &Value<'_>) -> Result<u8, PolicyDocumentError> {
    match (value.identifier, value.contents) {
        (ENUMERATED, [number]) => Ok(*number),
        _ => Err(PolicyDocumentError::Malformed("an enumeration")),
    }
}

/// Clause 6.10's `AlgorithmConstraints`.
fn read_algorithms<'a>(
    value: &Value<'a>,
    extensions: &mut Vec<&'a [u8]>,
    budget: &mut Budget,
) -> Result<Vec<AlgorithmAndLength<'a>>, PolicyDocumentError> {
    if value.identifier != SEQUENCE {
        return Err(PolicyDocumentError::Malformed("AlgorithmConstraints"));
    }
    let mut out = Vec::new();
    let mut entries = value.children()?;
    while let Some(entry) = entries.next_value()? {
        budget.take()?;
        let entry = expect(Some(entry), SEQUENCE, "AlgAndLength")?;
        let mut members = entry.children()?;
        let algorithm = members
            .next_value()?
            .and_then(|first| first.object_identifier())
            .ok_or(PolicyDocumentError::Malformed("AlgAndLength.algID"))?;
        let mut min_key_length = None;
        while let Some(member) = members.next_value()? {
            if member.identifier == der::INTEGER {
                min_key_length = Some(key_length(member.contents)?);
            } else {
                read_extensions(&member, extensions, budget)?;
            }
        }
        out.push(AlgorithmAndLength {
            algorithm,
            min_key_length,
        });
    }
    Ok(out)
}

/// `minKeyLength`, a non-negative `INTEGER` that fits in 32 bits.
fn key_length(contents: &[u8]) -> Result<u32, PolicyDocumentError> {
    let malformed = PolicyDocumentError::Malformed("AlgAndLength.minKeyLength");
    let digits = match contents {
        [first, ..] if first & 0x80 != 0 => return Err(malformed),
        [0x00, rest @ ..] => rest,
        all => all,
    };
    if digits.len() > 4 {
        return Err(malformed);
    }
    Ok(digits
        .iter()
        .fold(0, |value, digit| value << 8 | u32::from(*digit)))
}

fn read_commitment_rule<'a>(
    value: &Value<'a>,
    extensions: &mut Vec<&'a [u8]>,
    budget: &mut Budget,
) -> Result<CommitmentRule<'a>, PolicyDocumentError> {
    let value = expect(Some(*value), SEQUENCE, "CommitmentRule")?;
    let mut members = value.children()?;
    let types = expect(members.next_value()?, SEQUENCE, "selCommitmentTypes")?;
    let mut selected = Vec::new();
    let mut entries = types.children()?;
    while let Some(entry) = entries.next_value()? {
        budget.take()?;
        selected.push(match entry.identifier {
            der::NULL => None,
            SEQUENCE => Some(
                entry
                    .children()?
                    .next_value()?
                    .and_then(|first| first.object_identifier())
                    .ok_or(PolicyDocumentError::Malformed("CommitmentType.identifier"))?,
            ),
            _ => return Err(PolicyDocumentError::Malformed("selCommitmentTypes")),
        });
    }
    let rules = read_rules(&mut members, extensions, budget)?;
    Ok(CommitmentRule { selected, rules })
}

/// The instant a signature is judged as made at, and where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SigningTime {
    /// The instant.
    pub instant: Instant,
    /// The instant as its source spells it.
    pub spelled: String,
    /// Where it came from.
    pub source: TimeSource,
}

/// Where a [`SigningTime`] came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeSource {
    /// The `genTime` of a signature timestamp whose imprint covers this signature.
    SignatureTimestamp,
    /// A time the signer stated, which proves nothing about when the signature existed.
    Claimed,
}

/// What enforcement needs to know about a signature that the `SignedData` alone does not say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignerFacts {
    /// When the signature was made, as far as anything establishes it.
    pub signing_time: Option<SigningTime>,
    /// The signer's public key length in bits, where its family has one this reader measures.
    pub key_bits: Option<usize>,
    /// Whether the `SignedData` carries the certificate its `SignerInfo` names.
    pub carries_signer_certificate: bool,
}

impl SignerFacts {
    /// The facts a `SignedData` states, with the signing time its signature timestamp claims
    /// where that timestamp's imprint covers this signature.
    #[must_use]
    pub fn of(cms: &SignedData<'_>) -> Self {
        let certificate =
            crate::signature::signer_certificate(cms).and_then(|value| x509::read(value).ok());
        let key_bits = certificate.and_then(|certificate| match certificate.public_key {
            PublicKey::Rsa(key) => Some(key.bits()),
            PublicKey::Dsa(key) => Some(key.bits()),
            PublicKey::Ec(key) => Some(key.curve.bits()),
            PublicKey::EdDsa(_)
            | PublicKey::EcCurveNotVerifiable { .. }
            | PublicKey::Unverifiable { .. } => None,
        });
        let signing_time = match crate::timestamp::signature_timestamp(cms) {
            Some(Ok(stamp)) if stamp.covers_the_signature => Some(SigningTime {
                instant: stamp.claim.gen_time,
                spelled: stamp.claim.stated.clone(),
                source: TimeSource::SignatureTimestamp,
            }),
            _ => None,
        };
        Self {
            signing_time,
            key_bits,
            carries_signer_certificate: certificate.is_some(),
        }
    }
}

/// Whether one constraint held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The signature meets it.
    Met,
    /// The signature breaks it.
    Broken,
    /// This program did not evaluate it, and the constraint says why.
    NotEvaluated,
}

/// One constraint of a policy, as it applies to one signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Constraint {
    /// Clause 6.2's signing period.
    SigningPeriod {
        /// When the signature was made, where anything establishes it.
        at: Option<SigningTime>,
        /// `notBefore`.
        not_before: Stated,
        /// `notAfter`.
        not_after: Option<Stated>,
    },
    /// A signed attribute clause 6.5.1's `mandatedSignedAttr` names.
    SignedAttribute(String),
    /// An unsigned attribute clause 6.5.1's `mandatedUnsignedAttr` names.
    UnsignedAttribute(String),
    /// An unsigned attribute clause 6.5.2 asks a verifier to add where the signer did not.
    VerifierAdds(String),
    /// Clause 6.5.1's `externalSignedData`.
    ExternalSignedData {
        /// What the policy requires.
        required: bool,
    },
    /// Clause 6.5.1's `mandatedCertificateRef`.
    CertificateReference(CertificateReference),
    /// Clause 6.5.1's `mandatedCertificateInfo`.
    CertificateInfo(CertificateInfo),
    /// One algorithm the signer used, against clause 6.10's `signerAlgorithmConstraints`.
    SignerAlgorithm {
        /// The algorithm, dotted.
        algorithm: String,
        /// Whether the list names it.
        listed: bool,
        /// The list's minimum key length for it, where it states one and the algorithm is the
        /// signature algorithm.
        min_key_length: Option<u32>,
        /// The signer's key length, where measured.
        key_bits: Option<usize>,
    },
    /// Clause 6.4's commitment-type selection.
    CommitmentType {
        /// The commitment type the signature states, dotted.
        stated: Option<String>,
    },
    /// A trust condition of clauses 6.7 to 6.9, named.
    TrustCondition(&'static str),
    /// One of clause 6.10's certificate-issuer constraint lists, named.
    CertificateAlgorithms(&'static str),
    /// A clause 6.11 extension, by its identifier.
    Extension(String),
}

/// One constraint and its verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The verdict.
    pub verdict: Verdict,
    /// The constraint.
    pub constraint: Constraint,
}

/// Why a policy's constraints were not enforced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotEnforced {
    /// The file carries no copy of the policy document.
    NoStoredDocument,
    /// The stored copy is not bound to the signature: the binding said so, in its own words.
    NotBound(Binding),
    /// The stored copy is bound but its constraints could not be read.
    Unreadable(PolicyDocumentError),
    /// The stored document is a different policy from the one the signer named.
    IdentifierDiffers {
        /// The identifier the document states, dotted.
        document: String,
    },
}

/// What came of enforcing a signature's policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Enforcement {
    /// The constraints were read and each became a finding.
    Enforced(Vec<Finding>),
    /// They were not, and why.
    NotEnforced(NotEnforced),
}

/// §12.8.3.4.4's "shall enforce signature policy constraints", for a policy stored in the file
/// under ETSI TR 102 272's ASN.1 syntax.
///
/// The document is enforced only where it is bound to the signature — its octets digest to the
/// value the signer signed, or (ETSI TR 102 272 Annex B.11's second way of obtaining that value)
/// the signer signed the policy's own hash, which [`PolicyDocument::parse`] has re-computed — and
/// only where it names the policy the signer named. A store is unsigned, so a document neither
/// binds is one anybody could have put there, and its rules are nobody's agreement.
#[must_use]
pub fn enforce(
    policy: &SignaturePolicy<'_>,
    cms: &SignedData<'_>,
    facts: &SignerFacts,
) -> Enforcement {
    let Some(Store::Document { octets, .. }) = &policy.store else {
        return Enforcement::NotEnforced(NotEnforced::NoStoredDocument);
    };
    let binding = policy.binding();
    let parsed = PolicyDocument::parse(octets);
    let bound_by_own_hash = match (&policy.hash, &parsed) {
        (PolicyHash::Stated { digest, value }, Ok(document)) => {
            document.own_hash.is_some_and(|(algorithm, own)| {
                Digest::from_oid(algorithm) == Some(*digest) && own == *value
            })
        }
        _ => false,
    };
    if !matches!(binding, Binding::Matches { .. }) && !bound_by_own_hash {
        return Enforcement::NotEnforced(NotEnforced::NotBound(binding));
    }
    let document = match parsed {
        Ok(document) => document,
        Err(refusal) => return Enforcement::NotEnforced(NotEnforced::Unreadable(refusal)),
    };
    if name(document.identifier) != policy.identifier {
        return Enforcement::NotEnforced(NotEnforced::IdentifierDiffers {
            document: name(document.identifier),
        });
    }
    Enforcement::Enforced(findings(&document, cms, facts))
}

/// Every constraint of a parsed document, judged against one signature.
#[must_use]
pub fn findings(
    document: &PolicyDocument<'_>,
    cms: &SignedData<'_>,
    facts: &SignerFacts,
) -> Vec<Finding> {
    let mut out = Vec::new();
    let period = match &facts.signing_time {
        None => Verdict::NotEvaluated,
        Some(at) => {
            let before = at.instant < document.not_before.instant;
            let after = document
                .not_after
                .as_ref()
                .is_some_and(|end| at.instant > end.instant);
            if before || after {
                Verdict::Broken
            } else {
                Verdict::Met
            }
        }
    };
    out.push(Finding {
        verdict: period,
        constraint: Constraint::SigningPeriod {
            at: facts.signing_time.clone(),
            not_before: document.not_before.clone(),
            not_after: document.not_after.clone(),
        },
    });
    let stated = cms
        .attribute(cms::ID_AA_ETS_COMMITMENT_TYPE)
        .filter(|attribute| attribute.signed)
        .and_then(|attribute| attribute.first)
        .and_then(|value| value.children().ok()?.next_value().ok()?)
        .and_then(|first| first.object_identifier());
    let rule = document.commitment_rule(stated);
    if rule.is_err() || !document.commitment_rules.is_empty() {
        out.push(Finding {
            verdict: if rule.is_ok() {
                Verdict::Met
            } else {
                Verdict::Broken
            },
            constraint: Constraint::CommitmentType {
                stated: stated.map(name),
            },
        });
    }
    let rule = rule.ok().flatten();
    judge(&document.common, cms, facts, &mut out);
    if let Some(rule) = rule {
        judge(&rule.rules, cms, facts, &mut out);
    }
    for extension in &document.extensions {
        out.push(Finding {
            verdict: Verdict::NotEvaluated,
            constraint: Constraint::Extension(name(extension)),
        });
    }
    out
}

/// One rule set's findings.
fn judge(rules: &Rules<'_>, cms: &SignedData<'_>, facts: &SignerFacts, out: &mut Vec<Finding>) {
    let verdict = |held: bool| if held { Verdict::Met } else { Verdict::Broken };
    if let Some(signer) = &rules.signer {
        if let Some(required) = signer.external_signed_data {
            out.push(Finding {
                verdict: verdict(cms.encapsulated.is_none() == required),
                constraint: Constraint::ExternalSignedData { required },
            });
        }
        for oid in &signer.mandated_signed {
            out.push(Finding {
                verdict: verdict(cms.has_signed_attribute(oid)),
                constraint: Constraint::SignedAttribute(name(oid)),
            });
        }
        for oid in &signer.mandated_unsigned {
            out.push(Finding {
                verdict: verdict(cms.has_unsigned_attribute(oid)),
                constraint: Constraint::UnsignedAttribute(name(oid)),
            });
        }
        let reference = match signer.certificate_reference {
            CertificateReference::SignerOnly => {
                verdict(cms.signing_certificate.is_some() || cms.signing_certificate_v2.is_some())
            }
            CertificateReference::FullPath => Verdict::NotEvaluated,
        };
        out.push(Finding {
            verdict: reference,
            constraint: Constraint::CertificateReference(signer.certificate_reference),
        });
        let info = match signer.certificate_info {
            CertificateInfo::None => None,
            CertificateInfo::SignerOnly => Some(verdict(facts.carries_signer_certificate)),
            CertificateInfo::FullPath => Some(Verdict::NotEvaluated),
        };
        if let Some(info) = info {
            out.push(Finding {
                verdict: info,
                constraint: Constraint::CertificateInfo(signer.certificate_info),
            });
        }
    }
    for oid in rules.verifier_adds.iter().flatten() {
        out.push(Finding {
            verdict: Verdict::NotEvaluated,
            constraint: Constraint::VerifierAdds(name(oid)),
        });
    }
    for condition in &rules.trust_conditions {
        out.push(Finding {
            verdict: Verdict::NotEvaluated,
            constraint: Constraint::TrustCondition(condition),
        });
    }
    if let Some(list) = &rules.signer_algorithms {
        for (used, is_signature) in [
            (cms.digest_algorithm, false),
            (cms.signature_algorithm, true),
        ] {
            let entry = list.iter().find(|entry| entry.algorithm == used);
            let min_key_length = entry
                .and_then(|entry| entry.min_key_length)
                .filter(|_| is_signature);
            let verdict = match (entry, min_key_length, facts.key_bits) {
                (None, _, _) => Verdict::Broken,
                (Some(_), None, _) => Verdict::Met,
                (Some(_), Some(_), None) => Verdict::NotEvaluated,
                (Some(_), Some(minimum), Some(bits)) => {
                    verdict(usize::try_from(minimum).is_ok_and(|minimum| bits >= minimum))
                }
            };
            out.push(Finding {
                verdict,
                constraint: Constraint::SignerAlgorithm {
                    algorithm: name(used),
                    listed: entry.is_some(),
                    min_key_length,
                    key_bits: facts.key_bits,
                },
            });
        }
    }
    for member in &rules.certificate_algorithms {
        out.push(Finding {
            verdict: Verdict::NotEvaluated,
            constraint: Constraint::CertificateAlgorithms(member),
        });
    }
}

impl std::fmt::Display for Finding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let verdict = match self.verdict {
            Verdict::Met => "met",
            Verdict::Broken => "broken",
            Verdict::NotEvaluated => "not evaluated",
        };
        match &self.constraint {
            Constraint::SigningPeriod {
                at,
                not_before,
                not_after,
            } => period_sentence(f, verdict, at.as_ref(), not_before, not_after.as_ref()),
            Constraint::SignedAttribute(oid) => write!(
                f,
                "the policy's mandated signed attribute {oid} is {verdict}"
            ),
            Constraint::UnsignedAttribute(oid) => write!(
                f,
                "the policy's mandated unsigned attribute {oid} is {verdict}"
            ),
            Constraint::VerifierAdds(oid) => write!(
                f,
                "the policy asks a verifier to add unsigned attribute {oid} where the signer did not, and that is {verdict}: this program adds nothing to a signature"
            ),
            Constraint::ExternalSignedData { required } => write!(
                f,
                "the policy's requirement that the signed data be {} the CMS object is {verdict}",
                if *required { "outside" } else { "inside" }
            ),
            Constraint::CertificateReference(reference) => match reference {
                CertificateReference::SignerOnly => write!(
                    f,
                    "the policy's requirement that the signer reference their certificate in a signing-certificate attribute is {verdict}"
                ),
                CertificateReference::FullPath => write!(
                    f,
                    "the policy's requirement that the signing-certificate attribute reference every certificate up to a trust point is {verdict}: the path is the trust anchors' question, which a host answers"
                ),
            },
            Constraint::CertificateInfo(info) => match info {
                CertificateInfo::FullPath => write!(
                    f,
                    "the policy's requirement that the signature carry every certificate up to a trust point is {verdict}: the path is the trust anchors' question, which a host answers"
                ),
                _ => write!(
                    f,
                    "the policy's requirement that the signature carry the signer's certificate is {verdict}"
                ),
            },
            Constraint::SignerAlgorithm {
                algorithm,
                listed,
                min_key_length,
                key_bits,
            } => {
                if !listed {
                    return write!(
                        f,
                        "the policy's signer algorithm constraints are {verdict}: they do not admit {algorithm}"
                    );
                }
                match (min_key_length, key_bits) {
                    (Some(minimum), Some(bits)) => write!(
                        f,
                        "the policy's signer algorithm constraint on {algorithm}, a key of at least {minimum} bits, is {verdict} by a key of {bits} bits"
                    ),
                    (Some(minimum), None) => write!(
                        f,
                        "the policy's signer algorithm constraint on {algorithm}, a key of at least {minimum} bits, is {verdict}: this program does not measure the signer's key"
                    ),
                    (None, _) => write!(
                        f,
                        "the policy's signer algorithm constraints admit {algorithm}, so that constraint is {verdict}"
                    ),
                }
            }
            Constraint::CommitmentType { stated } => match stated {
                Some(oid) => write!(
                    f,
                    "the policy's commitment rules, for commitment type {oid}, are {verdict}"
                ),
                None => write!(
                    f,
                    "the policy's commitment rules, for a signature stating no commitment type, are {verdict}"
                ),
            },
            Constraint::TrustCondition(condition) => write!(
                f,
                "the policy's {condition} is {verdict}: its trust points are not the anchors this program validates under, which a host supplies"
            ),
            Constraint::CertificateAlgorithms(member) => write!(
                f,
                "the policy's {member} is {verdict}: it constrains the issuers along the certification path"
            ),
            Constraint::Extension(oid) => write!(
                f,
                "the policy's extension {oid} is {verdict}: its syntax is whatever its identifier names"
            ),
        }
    }
}

/// [`Constraint::SigningPeriod`]'s sentence.
fn period_sentence(
    f: &mut std::fmt::Formatter<'_>,
    verdict: &str,
    at: Option<&SigningTime>,
    not_before: &Stated,
    not_after: Option<&Stated>,
) -> std::fmt::Result {
    write!(
        f,
        "the policy's signing period, from {}",
        not_before.spelled
    )?;
    if let Some(end) = not_after {
        write!(f, " to {}", end.spelled)?;
    }
    match at {
        Some(at) => write!(
            f,
            ", is {verdict} by a signature made at {} as {}",
            at.spelled,
            match at.source {
                TimeSource::SignatureTimestamp => "its signature timestamp states",
                TimeSource::Claimed => "its signer claims",
            }
        ),
        None => write!(
            f,
            ", is {verdict}: nothing establishes when the signature was made"
        ),
    }
}

#[cfg(test)]
mod tests;
