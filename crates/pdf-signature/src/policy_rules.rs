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
//! certificate reference and information requirements, the commitment-type selection of clause
//! 6.4, and clause 6.10's signer algorithm constraints with their minimum key length. **The
//! constraints on the certification path are evaluated over a path built to the policy's own trust
//! points** ([`certification_path`], ADR 2714) — those points, not the anchors a host supplies
//! (ADR 1076), are what clause 6.7's signing-certificate trust condition states: the condition
//! itself with each trust point's `pathLenConstraint`, the `fullPath` certificate reference and
//! information requirements of clause 6.5.1, and clause 6.10's `eeCertAlgorithmConstraints` and
//! `caCertAlgorithmConstraints` against the issuers along that path. Clause 6.8's time-stamp trust
//! condition is evaluated the same way on a path from the certificate of the authority that issued
//! the signature's time-stamp token, with clause 6.10's `tsaCertAlgorithmConstraints` on that
//! certificate, and clause 6.6.2's revocation checks of both conditions are answered per kind of
//! certificate from the material beside the signature ([`revocation_checks`], ADR 2720). Named and
//! not evaluated: a trust point's acceptable policy set, name constraints and policy constraints,
//! which are RFC 5280 section 6.1's policy tree and name processing that [`crate::trust`] does not
//! perform, and the authority's name constraints, caution period and delay; the attribute trust
//! condition of clause 6.9 and clause 6.10's attribute-authority list, which concern attribute
//! certificates this program does not read; the verifier rules of clause 6.5.2, which ask a
//! verifier to *add* attributes and so are an augmentation this reader never performs; and every
//! extension of clause 6.11, whose syntax is whatever its own identifier names.
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
//!
//! # The other syntaxes
//!
//! ETSI TS 119 172-3 writes the same family of constraints in a second ASN.1 syntax, whose top
//! level opens with a digest of its components where this one opens with an algorithm identifier,
//! and ETSI TS 119 172-2 writes TS 119 172-3's building blocks in XML. [`Syntax::of`] tells the
//! three apart — markup first, then the first member of the ASN.1 top level — [`enforce`] reads
//! each with its own parser, [`ts_119_172_3`] for the second and [`ts_119_172_2`] for the third,
//! and all three speak the one [`Finding`] vocabulary the verdict takes (ADRs 2580, 2604).

use crate::cms::{self, Digest, SignedData};
use crate::der::{self, DerError, NotCanonical, OBJECT_IDENTIFIER, OCTET_STRING, SEQUENCE, Value};
use crate::policy::{Binding, PolicyHash, Published, SignaturePolicy, Store};
use crate::revocation::Material;
use crate::signature::name;
use crate::x509::{self, Instant, PublicKey};

pub mod canonical_xml;
pub mod certification_path;
pub mod revocation_checks;
pub mod ts_119_172_2;
pub mod ts_119_172_3;

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
/// Clause 6.10's list for the issuers of end-entity certificates.
pub const END_ENTITY_ALGORITHMS: &str = "eeCertAlgorithmConstraints";
/// Clause 6.10's list for the issuers of CA certificates.
pub const AUTHORITY_ALGORITHMS: &str = "caCertAlgorithmConstraints";
/// Clause 6.10's list for the certificates of time-stamping authorities.
pub const TSA_ALGORITHMS: &str = "tsaCertAlgorithmConstraints";

/// One rule of clause 6.3's placement: its name, whether a rule set states it, and whether it is
/// one of the three that must be stated somewhere.
type Placement = (&'static str, fn(&Rules<'_>) -> bool, bool);

/// `BOOLEAN`, primitive and universal.
const BOOLEAN: u8 = 0x01;
/// `ENUMERATED`, primitive and universal.
const ENUMERATED: u8 = 0x0A;
/// `GeneralizedTime`, primitive and universal.
const GENERALIZED_TIME: u8 = 0x18;

/// The machine-processable signature policy syntax a document is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Syntax {
    /// ETSI TR 102 272 clause 6's `SignaturePolicy`, the syntax RFC 3125 also publishes.
    Tr102272,
    /// ETSI TS 119 172-3 clause 4.2's `SignaturePolicy`.
    Ts1191723,
    /// ETSI TS 119 172-2 clause 4.2's `SignaturePolicy` element.
    Ts1191722,
}

impl Syntax {
    /// The syntax a document is written in: XML where it opens with markup
    /// ([`ts_119_172_2::is_xml`]), else read from the ASN.1 top level's first member's first
    /// value — TR 102 272's algorithm identifier opens with an object identifier, TS 119 172-3's
    /// `Digest` with a sequence. `None` where it is none of them, which each parser refuses by
    /// name.
    #[must_use]
    pub fn of(octets: &[u8]) -> Option<Self> {
        if octets.len() > MAX_POLICY_OCTETS {
            return None;
        }
        if ts_119_172_2::is_xml(octets) {
            return Some(Self::Ts1191722);
        }
        let policy = der::Reader::new(octets).ok()?.next_value().ok()??;
        let first = policy.children().ok()?.next_value().ok()??;
        let inner = first.children().ok()?.next_value().ok()??;
        match inner.identifier {
            OBJECT_IDENTIFIER => Some(Self::Tr102272),
            SEQUENCE => Some(Self::Ts1191723),
            _ => None,
        }
    }
}

impl std::fmt::Display for Syntax {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Tr102272 => "ETSI TR 102 272's ASN.1 syntax",
            Self::Ts1191723 => "ETSI TS 119 172-3's ASN.1 syntax",
            Self::Ts1191722 => "ETSI TS 119 172-2's XML syntax",
        })
    }
}

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
    /// The document is not, at its top level, the syntax its parser reads, which is named.
    ///
    /// The two machine-processable ASN.1 syntaxes are told apart by their first member rather
    /// than one read as the other (see [`Syntax::of`]).
    #[error("the policy document is not written in {0}")]
    OtherSyntax(Syntax),
    /// A member clause 6 requires is missing or of another type; the member is named.
    #[error(
        "the policy document's {0} is missing or is not the type ETSI TR 102 272 clause 6 gives it"
    )]
    Malformed(&'static str),
    /// A member ETSI TS 119 172-3 clause 4 requires is missing or of another type; the member is
    /// named.
    #[error(
        "the policy document's {0} is missing or is not the type ETSI TS 119 172-3 clause 4 gives it"
    )]
    ComponentMalformed(&'static str),
    /// The XML of a TS 119 172-2 document cannot be read: it is not UTF-8 or not well-formed, or
    /// it declares a document type; the reason is said.
    #[error("the policy document's XML cannot be read: {0}")]
    Xml(String),
    /// A TS 119 172-2 document nests elements deeper than [`der::MAX_DEPTH`], the bound the ASN.1
    /// syntaxes' values are held to.
    #[error(
        "the policy document nests its elements deeper than the {} levels this program reads",
        der::MAX_DEPTH
    )]
    NestedTooDeep,
    /// An element ETSI TS 119 172-2 clause 4 requires is missing or its value is not of the type
    /// the clause gives it; the element is named.
    #[error(
        "the policy document's {0} is missing or is not what ETSI TS 119 172-2 clause 4 gives it"
    )]
    ElementMalformed(&'static str),
    /// TS 119 172-3 clause 4.3.2's digest, re-computed over the policy components, is not the
    /// value the document states.
    #[error(
        "the policy document's digest, re-computed over its policy components, is not the value it states"
    )]
    DigestDisagrees,
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
    /// Clause 6.7's `signingCertTrustCondition`, where stated: its trust trees, and whether its
    /// `signerRevReq` mandates any check.
    pub signer_trust: Option<SignerTrust<'a>>,
    /// Clause 6.10's `eeCertAlgorithmConstraints`, where stated.
    pub end_entity_algorithms: Option<Vec<AlgorithmAndLength<'a>>>,
    /// Clause 6.10's `caCertAlgorithmConstraints`, where stated.
    pub authority_algorithms: Option<Vec<AlgorithmAndLength<'a>>>,
    /// Clause 6.8's `timeStampTrustCondition`, where stated.
    pub timestamp_trust: Option<TimestampTrust<'a>>,
    /// Clause 6.10's `tsaCertAlgorithmConstraints`, where stated.
    pub authority_certificate_algorithms: Option<Vec<AlgorithmAndLength<'a>>>,
    /// Whether clause 6.10's `aaCertAlgorithmConstraints` is stated: it constrains attribute
    /// certificates, which this program does not read.
    pub attribute_algorithms: bool,
}

/// Clause 6.7's `SigningCertTrustCondition`, as far as it is evaluated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignerTrust<'a> {
    /// `signerTrustTrees`.
    pub trees: certification_path::TrustTrees<'a>,
    /// `signerRevReq`: the check each kind of certificate on the path is to have.
    pub revocation: revocation_checks::Requirement,
}

/// Clause 6.8's `TimestampTrustCondition`, or the trust condition of TS 119 172-2 clause 4.25.1
/// item 3, as far as it is evaluated (ADR 2720).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimestampTrust<'a> {
    /// `ttsCertificateTrustTrees`, where stated. Where it is not, clause 6.8 applies the
    /// signing-certificate condition's trees to the authority; TS 119 172-2 states no such rule.
    pub trees: Option<certification_path::TrustTrees<'a>>,
    /// `ttsRevReq`, where stated.
    pub revocation: Option<revocation_checks::Requirement>,
    /// `ttsNameConstraints`, where stated: constraints on the authority's own name (RFC 3125
    /// section 6.8), judged on the certificate of the authority whose path the condition searched.
    pub names: Option<crate::trust::names::NameConstraints>,
    /// The members this program does not evaluate, by name: clause 6.8's caution period and
    /// signature time-stamp delay.
    pub unprocessed: Vec<&'static str>,
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
        if ts_119_172_2::is_xml(octets) {
            return Err(PolicyDocumentError::OtherSyntax(Syntax::Tr102272));
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
            .ok_or(PolicyDocumentError::OtherSyntax(Syntax::Tr102272))?;
        // An `AlgorithmIdentifier` opens with its object identifier; TS 119 172-3's `Digest`
        // opens with an `AlgorithmIdentifier` instead, which is how the two are told apart.
        let algorithm = algorithm
            .children()?
            .next_value()?
            .and_then(|first| first.object_identifier())
            .ok_or(PolicyDocumentError::OtherSyntax(Syntax::Tr102272))?;
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
            1 => {
                rules.trust_conditions.push(SIGNING_CERT_TRUST);
                let condition = expect(
                    Some(explicit(&member, "signingCertTrustCondition")?),
                    SEQUENCE,
                    "signingCertTrustCondition",
                )?;
                rules.signer_trust = Some(read_signer_trust(&condition, extensions, budget)?);
            }
            2 => {
                rules.trust_conditions.push(TIMESTAMP_TRUST);
                rules.timestamp_trust = Some(read_timestamp_trust(&member, extensions, budget)?);
            }
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
                        (der::Class::ContextSpecific, 1) => {
                            rules.end_entity_algorithms = Some(read_algorithms(
                                &explicit(&list, END_ENTITY_ALGORITHMS)?,
                                extensions,
                                budget,
                            )?);
                        }
                        (der::Class::ContextSpecific, 2) => {
                            rules.authority_algorithms = Some(read_algorithms(
                                &explicit(&list, AUTHORITY_ALGORITHMS)?,
                                extensions,
                                budget,
                            )?);
                        }
                        (der::Class::ContextSpecific, 3) => {
                            let member = "aaCertAlgorithmConstraints";
                            read_algorithms(&explicit(&list, member)?, extensions, budget)?;
                            rules.attribute_algorithms = true;
                        }
                        (der::Class::ContextSpecific, 4) => {
                            rules.authority_certificate_algorithms = Some(read_algorithms(
                                &explicit(&list, TSA_ALGORITHMS)?,
                                extensions,
                                budget,
                            )?);
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

/// Clause 6.7's `SigningCertTrustCondition`: its trust trees, and whether its revocation
/// requirement mandates a check.
fn read_signer_trust<'a>(
    value: &Value<'a>,
    extensions: &mut Vec<&'a [u8]>,
    budget: &mut Budget,
) -> Result<SignerTrust<'a>, PolicyDocumentError> {
    let mut members = value.children()?;
    let trees = expect(members.next_value()?, SEQUENCE, "signerTrustTrees")?;
    let trees = read_trust_trees(&trees, budget)?;
    let revocation = read_revocation_requirement(
        &expect(members.next_value()?, SEQUENCE, "signerRevReq")?,
        extensions,
        budget,
    )?;
    Ok(SignerTrust { trees, revocation })
}

/// Clause 6.8's `TimestampTrustCondition`, under its rule's context tag: its trust trees and
/// revocation requirement, and the members this program does not evaluate by name.
fn read_timestamp_trust<'a>(
    tagged: &Value<'a>,
    extensions: &mut Vec<&'a [u8]>,
    budget: &mut Budget,
) -> Result<TimestampTrust<'a>, PolicyDocumentError> {
    let value = expect(
        Some(explicit(tagged, "timeStampTrustCondition")?),
        SEQUENCE,
        "timeStampTrustCondition",
    )?;
    let mut condition = TimestampTrust {
        trees: None,
        revocation: None,
        names: None,
        unprocessed: Vec::new(),
    };
    let mut members = value.children()?;
    while let Some(member) = members.next_value()? {
        if member.class() != der::Class::ContextSpecific {
            return Err(PolicyDocumentError::Malformed("TimestampTrustCondition"));
        }
        match member.tag_number() {
            0 => {
                let trees = expect(
                    Some(explicit(&member, "ttsCertificateTrustTrees")?),
                    SEQUENCE,
                    "ttsCertificateTrustTrees",
                )?;
                condition.trees = Some(read_trust_trees(&trees, budget)?);
            }
            1 => {
                let requirement =
                    expect(Some(explicit(&member, "ttsRevReq")?), SEQUENCE, "ttsRevReq")?;
                condition.revocation = Some(read_revocation_requirement(
                    &requirement,
                    extensions,
                    budget,
                )?);
            }
            2 => {
                condition.names = authority_names(
                    &explicit(&member, "ttsNameConstraints")?,
                    &mut condition.unprocessed,
                )?;
            }
            3 => condition.unprocessed.push("caution period"),
            4 => condition.unprocessed.push("signature time-stamp delay"),
            _ => return Err(PolicyDocumentError::Malformed("TimestampTrustCondition")),
        }
    }
    Ok(condition)
}

/// Clause 6.6.2's `CertRevReq`: the end certificate's check and the CA certificates'.
fn read_revocation_requirement<'a>(
    value: &Value<'a>,
    extensions: &mut Vec<&'a [u8]>,
    budget: &mut Budget,
) -> Result<revocation_checks::Requirement, PolicyDocumentError> {
    let mut requirement = value.children()?;
    let end_entity = revocation_check(
        &expect(requirement.next_value()?, SEQUENCE, "endCertRevReq")?,
        extensions,
        budget,
    )?;
    let authorities = requirement
        .next_value()?
        .ok_or(PolicyDocumentError::Malformed("CertRevReq.caCerts"))?;
    let authorities = revocation_check(
        &expect(
            Some(explicit(&authorities, "caCerts")?),
            SEQUENCE,
            "caCerts",
        )?,
        extensions,
        budget,
    )?;
    Ok(revocation_checks::Requirement {
        end_entity,
        authorities,
        unprocessed: Vec::new(),
    })
}

/// Clause 6.6.2's `RevReq`: its `enuRevReq`, any extension charged as every other is.
fn revocation_check<'a>(
    value: &Value<'a>,
    extensions: &mut Vec<&'a [u8]>,
    budget: &mut Budget,
) -> Result<revocation_checks::Check, PolicyDocumentError> {
    let mut members = value.children()?;
    let check = enumerated(
        &members
            .next_value()?
            .ok_or(PolicyDocumentError::Malformed("RevReq.enuRevReq"))?,
    )?;
    let check = revocation_checks::Check::of_number(check)
        .ok_or(PolicyDocumentError::Malformed("RevReq.enuRevReq"))?;
    if let Some(extension) = members.next_value()? {
        read_extensions(&extension, extensions, budget)?;
    }
    Ok(check)
}

/// Clause 6.6.1's `CertificateTrustTrees`, each trust point charged to the budget.
///
/// `acceptablePolicySet`, `nameConstraints` and `policyConstraints` are the initial conditions of
/// RFC 5280 section 6.1.1's policy tree and name processing (RFC 3125 section 6.6.1), read into
/// [`certification_path::TrustPoint::inputs`] under the module's `EXPLICIT TAGS` (ADR 2724).
fn read_trust_trees<'a>(
    value: &Value<'a>,
    budget: &mut Budget,
) -> Result<certification_path::TrustTrees<'a>, PolicyDocumentError> {
    let mut trees = certification_path::TrustTrees::default();
    let mut points = value.children()?;
    while let Some(point) = points.next_value()? {
        budget.take()?;
        let point = expect(Some(point), SEQUENCE, "CertificateTrustPoint")?;
        let mut members = point.children()?;
        let certificate = expect(
            members.next_value()?,
            SEQUENCE,
            "CertificateTrustPoint.trustpoint",
        )?;
        let mut path_length = None;
        let mut inputs = crate::trust::PathInputs::default();
        let mut unprocessed = Vec::new();
        while let Some(member) = members.next_value()? {
            if member.class() != der::Class::ContextSpecific {
                return Err(PolicyDocumentError::Malformed("CertificateTrustPoint"));
            }
            match member.tag_number() {
                0 => {
                    let limit = expect(
                        Some(explicit(&member, "pathLenConstraint")?),
                        der::INTEGER,
                        "pathLenConstraint",
                    )?;
                    path_length =
                        Some(i64::from(key_length(limit.contents).map_err(|_| {
                            PolicyDocumentError::Malformed("pathLenConstraint")
                        })?));
                }
                number @ 1..=3 => {
                    trust_point_input(
                        &mut inputs,
                        &mut unprocessed,
                        number,
                        &explicit(&member, "CertificateTrustPoint")?,
                    )?;
                }
                _ => return Err(PolicyDocumentError::Malformed("CertificateTrustPoint")),
            }
        }
        trees.points.push(certification_path::TrustPoint {
            certificate: std::borrow::Cow::Borrowed(certificate.encoding()),
            path_length,
            reliable_until: None,
            inputs,
            unprocessed,
        });
    }
    Ok(trees)
}

/// A time-stamp trust condition's `ttsNameConstraints`, in the module's `EXPLICIT TAGS`; a subtree
/// stating a distance keeps the constraint named instead, as a trust point's does. Shared with ETSI
/// TS 119 172-3.
pub(super) fn authority_names(
    value: &Value<'_>,
    unprocessed: &mut Vec<&'static str>,
) -> Result<Option<crate::trust::names::NameConstraints>, PolicyDocumentError> {
    use crate::trust::names;
    match names::read_constraints(value.encoding(), names::Tagging::Explicit) {
        Ok(read) => Ok(Some(read)),
        Err(names::Unreadable::DistanceStated) => {
            unprocessed.push("name constraints");
            Ok(None)
        }
        Err(_) => Err(PolicyDocumentError::Malformed("ttsNameConstraints")),
    }
}

/// One of a `CertificateTrustPoint`'s members `[1]` to `[3]`, the value inside its `EXPLICIT` tag,
/// as RFC 5280 section 6.1.1's inputs: `acceptablePolicySet` is `user-initial-policy-set`,
/// `nameConstraints` the initial subtrees, and `policyConstraints` the two `SkipCerts` counts,
/// applied as though the trust point were certificate zero (RFC 3125 section 6.6.1). Shared with
/// ETSI TS 119 172-3, whose Annex A states the same members in the same `EXPLICIT TAGS`.
pub(super) fn trust_point_input(
    inputs: &mut crate::trust::PathInputs,
    unprocessed: &mut Vec<&'static str>,
    number: u8,
    value: &Value<'_>,
) -> Result<(), PolicyDocumentError> {
    use crate::trust::{names, policies};
    match number {
        1 => {
            inputs.policy_set = Some(
                policies::read_policy_set(value)
                    .map_err(|_| PolicyDocumentError::Malformed("acceptablePolicySet"))?,
            );
        }
        // A subtree stating a distance, which RFC 5280 section 4.2.1.10 leaves unused, keeps the
        // whole constraint by name rather than evaluating the rest of it as though it were all.
        2 => match names::read_constraints(value.encoding(), names::Tagging::Explicit) {
            Ok(read) => inputs.name_constraints = Some(read),
            Err(names::Unreadable::DistanceStated) => unprocessed.push("name constraints"),
            Err(_) => return Err(PolicyDocumentError::Malformed("nameConstraints")),
        },
        _ => {
            let stated =
                policies::read_policy_constraints(value.encoding(), names::Tagging::Explicit)
                    .map_err(|_| PolicyDocumentError::Malformed("policyConstraints"))?;
            inputs.require_explicit_policy = stated.require_explicit_policy;
            inputs.inhibit_policy_mapping = stated.inhibit_policy_mapping;
        }
    }
    Ok(())
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
pub struct SignerFacts<'m> {
    /// When the signature was made, as far as anything establishes it.
    pub signing_time: Option<SigningTime>,
    /// The signer's public key length in bits, where its family has one this reader measures.
    pub key_bits: Option<usize>,
    /// Whether the `SignedData` carries the certificate its `SignerInfo` names.
    pub carries_signer_certificate: bool,
    /// The signature algorithm that certificate's issuer signed it under, where it is carried.
    pub certificate_signature_algorithm: Option<Vec<u8>>,
    /// RFC 5280 section 6.1.1's input (b) for a path to the policy's own trust points, where a
    /// host gave one; [`certification_path`] falls back to a covering signature timestamp's
    /// instant, and to nothing.
    pub path_at: Option<Instant>,
    /// The revocation material a trust condition's mandated checks are answered from (ADR 2720):
    /// §12.8.3.3.2's attribute the signer signed, and §12.8.4's store where a caller adds it with
    /// [`Self::with_material`].
    pub material: Material<'m>,
}

impl<'m> SignerFacts<'m> {
    /// The facts a `SignedData` states, with the signing time its signature timestamp claims
    /// where that timestamp's imprint covers this signature, and the revocation material its
    /// signer's `adbe-revocationInfoArchival` attribute carries.
    #[must_use]
    pub fn of(cms: &SignedData<'m>) -> Self {
        let certificate =
            crate::signature::signer_certificate(cms).and_then(|value| x509::read(value).ok());
        let key_bits = certificate.and_then(|certificate| key_bits(&certificate.public_key));
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
            certificate_signature_algorithm: certificate
                .map(|certificate| certificate.signature_algorithm.to_vec()),
            path_at: None,
            material: crate::revocation::archived(cms),
        }
    }

    /// These facts with `material` added to the revocation material — §12.8.4's document security
    /// store, which the document holds and the `SignedData` does not.
    #[must_use]
    pub fn with_material(mut self, material: &Material<'m>) -> Self {
        self.material.absorb(material.clone());
        self
    }

    /// These facts with the instant a host validates paths at, which is the instant
    /// [`certification_path::search`] validates a path to the policy's trust points at.
    #[must_use]
    pub const fn validated_at(mut self, at: Instant) -> Self {
        self.path_at = Some(at);
        self
    }
}

/// A public key's length in bits, where its family has one this reader measures.
pub(crate) fn key_bits(key: &PublicKey<'_>) -> Option<usize> {
    match key {
        PublicKey::Rsa(key) => Some(key.bits()),
        PublicKey::Dsa(key) => Some(key.bits()),
        PublicKey::Ec(key) => Some(key.curve.bits()),
        PublicKey::EdDsa(_)
        | PublicKey::EcCurveNotVerifiable { .. }
        | PublicKey::Unverifiable { .. } => None,
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
    /// A trust condition of clauses 6.7 to 6.9, named and not evaluated.
    TrustCondition(&'static str),
    /// A trust condition's trust trees, searched for a path to the policy's own trust points.
    TrustPath {
        /// The condition, by [`SIGNING_CERT_TRUST`] and its siblings.
        condition: &'static str,
        /// What the search found.
        found: certification_path::Found,
        /// How many trust anchors were not searched.
        unexamined: usize,
    },
    /// One kind of certificate's revocation check a trust condition mandates (clause 6.6.2, TS
    /// 119 172-2 clause 4.23.2), answered over that condition's path (ADR 2720).
    RevocationCheck {
        /// The condition.
        condition: &'static str,
        /// What was mandated and what the material answered.
        check: revocation_checks::KindCheck,
    },
    /// A time-stamp trust condition's name constraints, judged on the authority's own certificate
    /// (RFC 3125 section 6.8, TS 119 172-2 clause 4.25.1 item 3 d), ADR 2724).
    AuthorityName {
        /// The condition.
        condition: &'static str,
        /// What the authority's names were found to be.
        outcome: certification_path::AuthorityName,
    },
    /// A member of a trust condition this program does not evaluate, by name.
    ConditionMember {
        /// The condition.
        condition: &'static str,
        /// The member.
        member: &'static str,
    },
    /// One certificate on the path against one of clause 6.10's certificate-issuer lists.
    IssuerAlgorithm {
        /// The list, by its member name.
        member: &'static str,
        /// The algorithm the certificate was issued under, dotted, and the issuer's key length;
        /// `None` where no certificate is judged.
        issued: Option<(String, Option<usize>)>,
        /// Whether a path to the policy's trust points validated, which is where the issuers
        /// are known from.
        path: bool,
        /// Whether the list names the algorithm.
        listed: bool,
        /// The list's minimum key length for it.
        min_key_length: Option<u32>,
    },
    /// One of clause 6.10's certificate-issuer constraint lists, named.
    CertificateAlgorithms(&'static str),
    /// A clause 6.11 extension, or a TS 119 172-3 `Other`, by its identifier.
    Extension(String),
    /// A constraint of TS 119 172-3's policy components with no counterpart in clause 6.
    PolicyComponent(ts_119_172_3::Constraint),
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
    /// The copy fetched from the URL ETSI EN 319 122-1 clause 5.2.9.2's qualifier names is not
    /// bound to the signature: [`SignaturePolicy::fetched_binding`] said so, in its own words.
    FetchedCopyNotBound(Published),
    /// The stored copy is bound but its constraints could not be read.
    Unreadable(PolicyDocumentError),
    /// The signer signed the digest a TS 119 172-2 document states over its canonicalised
    /// components, under a canonicalisation method none of the three Recommendations
    /// [`canonical_xml`] computes names, so the binding is unchecked rather than refused (ADRs
    /// 2604, 2644, 2710).
    CanonicalDigestNotComputed {
        /// The canonicalisation method the document names.
        canonicalization: String,
    },
    /// The signer signed the digest a TS 119 172-2 document states over its canonicalised
    /// components, and the components, canonicalised by the method the document names, do not
    /// digest to it: the document is not the one the signer committed to (ADR 2644).
    CanonicalDigestDiffers {
        /// The canonicalisation method the document names.
        canonicalization: String,
    },
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
    Enforced {
        /// The syntax the document is written in.
        syntax: Syntax,
        /// One finding per constraint.
        findings: Vec<Finding>,
    },
    /// They were not, and why.
    NotEnforced(NotEnforced),
}

/// §12.8.3.4.4's "shall enforce signature policy constraints", for a policy stored in the file
/// under any of the three machine-processable syntaxes, [`Syntax::of`] deciding which parser
/// reads it.
///
/// The document is enforced only where it is bound to the signature — its octets digest to the
/// value the signer signed, or the signer signed the digest the document states over its own
/// rules: ETSI TR 102 272 Annex B.11 gives that as the second way of obtaining the value, and a
/// TS 119 172-3 document's `Digest` covers every rule it states, so signing it commits to the same
/// rules (ADR 2580); each parser has re-computed that digest — and only where it names the policy
/// the signer named. A store is unsigned, so a document neither
/// binds is one anybody could have put there, and its rules are nobody's agreement.
#[must_use]
pub fn enforce(
    policy: &SignaturePolicy<'_>,
    cms: &SignedData<'_>,
    facts: &SignerFacts<'_>,
) -> Enforcement {
    let Some(Store::Document { octets, .. }) = &policy.store else {
        return Enforcement::NotEnforced(NotEnforced::NoStoredDocument);
    };
    let by_octets = match policy.binding() {
        Binding::Matches { .. } => Ok(()),
        binding => Err(NotEnforced::NotBound(binding)),
    };
    enforce_copy(policy, octets, by_octets, cms, facts)
}

/// [`enforce`] for a copy of the policy document fetched from the URL ETSI EN 319 122-1 clause
/// 5.2.9.2's qualifier names, rather than one clause 5.2.10's store carries.
///
/// **The same three readers, the same bindings and the same findings**, because a server is no
/// more signed than a store is: clause 5.2.10's note makes the digest the signer signed the one
/// thing that catches a substituted document, and that argument does not depend on where the
/// octets came from (ADR 1728). So the copy is bound by its octets through
/// [`SignaturePolicy::fetched_binding`] — made from the signature's own identifier, never from what
/// a host says the signature stated — or by the digest the document states over its own rules,
/// exactly as a stored copy is, and a copy neither binds is reported and never enforced
/// (ADR 2624).
#[must_use]
pub fn enforce_fetched(
    policy: &SignaturePolicy<'_>,
    copy: &[u8],
    cms: &SignedData<'_>,
    facts: &SignerFacts<'_>,
) -> Enforcement {
    let by_octets = match policy.fetched_binding(copy) {
        Published::Matches { .. } => Ok(()),
        binding => Err(NotEnforced::FetchedCopyNotBound(binding)),
    };
    enforce_copy(policy, copy, by_octets, cms, facts)
}

/// One copy of the policy document enforced, wherever it came from: `by_octets` is `Ok` where its
/// octets digest to the value the signer signed, and otherwise why they do not.
fn enforce_copy(
    policy: &SignaturePolicy<'_>,
    octets: &[u8],
    by_octets: Result<(), NotEnforced>,
    cms: &SignedData<'_>,
    facts: &SignerFacts<'_>,
) -> Enforcement {
    match Syntax::of(octets) {
        Some(Syntax::Ts1191723) => {
            return enforce_components(policy, octets, by_octets, cms, facts);
        }
        Some(Syntax::Ts1191722) => return enforce_xml(policy, octets, by_octets, cms, facts),
        _ => {}
    }
    let parsed = PolicyDocument::parse(octets);
    let bound_by_own_hash = match (&policy.hash, &parsed) {
        (PolicyHash::Stated { digest, value }, Ok(document)) => {
            document.own_hash.is_some_and(|(algorithm, own)| {
                Digest::from_oid(algorithm) == Some(*digest) && own == *value
            })
        }
        _ => false,
    };
    if let Err(why) = by_octets
        && !bound_by_own_hash
    {
        return Enforcement::NotEnforced(why);
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
    Enforcement::Enforced {
        syntax: Syntax::Tr102272,
        findings: findings(&document, cms, facts),
    }
}

/// [`enforce`] for a document in TS 119 172-3's syntax.
fn enforce_components(
    policy: &SignaturePolicy<'_>,
    octets: &[u8],
    by_octets: Result<(), NotEnforced>,
    cms: &SignedData<'_>,
    facts: &SignerFacts<'_>,
) -> Enforcement {
    let parsed = ts_119_172_3::PolicyDocument::parse(octets);
    let bound_by_its_digest = match (&policy.hash, &parsed) {
        (PolicyHash::Stated { digest, value }, Ok(document)) => {
            Digest::from_oid(document.digest.0) == Some(*digest) && document.digest.1 == *value
        }
        _ => false,
    };
    if let Err(why) = by_octets
        && !bound_by_its_digest
    {
        return Enforcement::NotEnforced(why);
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
    Enforcement::Enforced {
        syntax: Syntax::Ts1191723,
        findings: ts_119_172_3::findings(&document, cms, facts),
    }
}

/// [`enforce`] for a document in TS 119 172-2's XML syntax, bound by the digest of its octets or
/// by the digest its own `Digest` states over its canonicalised components, re-computed (ADRs
/// 2604, 2644).
fn enforce_xml(
    policy: &SignaturePolicy<'_>,
    octets: &[u8],
    by_octets: Result<(), NotEnforced>,
    cms: &SignedData<'_>,
    facts: &SignerFacts<'_>,
) -> Enforcement {
    let reading = ts_119_172_2::Reading::parse(octets);
    let parsed = reading
        .as_ref()
        .map_err(Clone::clone)
        .and_then(ts_119_172_2::Reading::document);
    if let Err(why) = by_octets {
        let (PolicyHash::Stated { digest, value }, Ok(document)) = (&policy.hash, &parsed) else {
            return Enforcement::NotEnforced(why);
        };
        if document.digest_value != *value {
            return Enforcement::NotEnforced(why);
        }
        if let Err(refusal) = canonical_binding(octets, document, *digest, value) {
            return Enforcement::NotEnforced(refusal);
        }
    }
    let document = match parsed {
        Ok(document) => document,
        Err(refusal) => return Enforcement::NotEnforced(NotEnforced::Unreadable(refusal)),
    };
    if document.identifier_oid() != Some(policy.identifier.as_str()) {
        return Enforcement::NotEnforced(NotEnforced::IdentifierDiffers {
            document: document.identifier.to_owned(),
        });
    }
    Enforcement::Enforced {
        syntax: Syntax::Ts1191722,
        findings: ts_119_172_2::findings(&document, cms, facts),
    }
}

/// TS 119 172-2 clause 4.3.2's binding, made where the signer signed the value a document's
/// `Digest` states: its `PolicyComponents` element canonicalised by the method the document
/// names, digested under the function the signer named, is that value.
///
/// The signer's function rather than the document's `DigestMethod`, because the signed value and
/// its function are the commitment and the document is unsigned until this succeeds, so nothing
/// the document states can choose what it is checked under (ADR 2644).
fn canonical_binding(
    octets: &[u8],
    document: &ts_119_172_2::PolicyDocument<'_>,
    digest: Digest,
    signed: &[u8],
) -> Result<(), NotEnforced> {
    let canonicalization = document.canonicalization;
    let Some(method) = canonical_xml::Method::of(canonicalization, document.prefix_list) else {
        return Err(NotEnforced::CanonicalDigestNotComputed {
            canonicalization: canonicalization.to_owned(),
        });
    };
    let apex = canonical_xml::Apex {
        depth: 1,
        namespaces: &ts_119_172_2::NAMESPACE,
        local: "PolicyComponents",
    };
    let mut hasher = digest.hasher();
    canonical_xml::canonicalize(octets, &apex, &method, &mut |piece| hasher.update(piece))
        .map_err(NotEnforced::Unreadable)?;
    if hasher.finish() == signed {
        Ok(())
    } else {
        Err(NotEnforced::CanonicalDigestDiffers {
            canonicalization: canonicalization.to_owned(),
        })
    }
}

/// Clause 6.2's signing period, and TS 119 172-2 clause 4.13.1's, against when the signature was
/// made.
fn signing_period(
    facts: &SignerFacts<'_>,
    not_before: &Stated,
    not_after: Option<&Stated>,
) -> Finding {
    let verdict = match &facts.signing_time {
        None => Verdict::NotEvaluated,
        Some(at) => {
            let before = at.instant < not_before.instant;
            let after = not_after.is_some_and(|end| at.instant > end.instant);
            if before || after {
                Verdict::Broken
            } else {
                Verdict::Met
            }
        }
    };
    Finding {
        verdict,
        constraint: Constraint::SigningPeriod {
            at: facts.signing_time.clone(),
            not_before: not_before.clone(),
            not_after: not_after.cloned(),
        },
    }
}

/// The commitment type a signature's signed commitment-type-indication attribute states.
fn stated_commitment_type<'a>(cms: &SignedData<'a>) -> Option<&'a [u8]> {
    cms.attribute(cms::ID_AA_ETS_COMMITMENT_TYPE)
        .filter(|attribute| attribute.signed)
        .and_then(|attribute| attribute.first)
        .and_then(|value| value.children().ok()?.next_value().ok()?)
        .and_then(|first| first.object_identifier())
}

/// Every constraint of a parsed document, judged against one signature.
#[must_use]
pub fn findings(
    document: &PolicyDocument<'_>,
    cms: &SignedData<'_>,
    facts: &SignerFacts<'_>,
) -> Vec<Finding> {
    let mut out = vec![signing_period(
        facts,
        &document.not_before,
        document.not_after.as_ref(),
    )];
    let stated = stated_commitment_type(cms);
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
    // Clause 6.3 places the signing-certificate and time-stamp trust conditions each in the common
    // rules or in every commitment rule, never both, so the one that applies is searched once and
    // every path-dependent constraint of either rule set reads that one path.
    let signer_trust = document
        .common
        .signer_trust
        .as_ref()
        .or_else(|| rule.and_then(|rule| rule.rules.signer_trust.as_ref()));
    let search = signer_trust.map(|trust| certification_path::search(&trust.trees, cms, facts));
    // Clause 6.8: where the time-stamp condition states no trees of its own, the signing-certificate
    // condition's apply to the authority's certificate.
    let authority = document
        .common
        .timestamp_trust
        .as_ref()
        .or_else(|| rule.and_then(|rule| rule.rules.timestamp_trust.as_ref()))
        .and_then(|stamp| {
            stamp
                .trees
                .as_ref()
                .or_else(|| signer_trust.map(|trust| &trust.trees))
        })
        .map(|trees| certification_path::search_authority(trees, cms, facts));
    let paths = Paths {
        signer: search.as_ref(),
        authority: authority.as_ref(),
    };
    judge(&document.common, cms, facts, paths, &mut out);
    if let Some(rule) = rule {
        judge(&rule.rules, cms, facts, paths, &mut out);
    }
    for extension in &document.extensions {
        out.push(Finding {
            verdict: Verdict::NotEvaluated,
            constraint: Constraint::Extension(name(extension)),
        });
    }
    out
}

/// The searches one document's trust conditions are judged on: the signer's, and the
/// time-stamping authority's (ADRs 2714, 2720).
#[derive(Debug, Clone, Copy)]
struct Paths<'s> {
    signer: Option<&'s certification_path::Search>,
    authority: Option<&'s certification_path::Search>,
}

/// One rule set's findings.
fn judge(
    rules: &Rules<'_>,
    cms: &SignedData<'_>,
    facts: &SignerFacts<'_>,
    paths: Paths<'_>,
    out: &mut Vec<Finding>,
) {
    let path = paths.signer.and_then(|search| search.path.as_ref());
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
            CertificateReference::FullPath => match path.map(|path| path.referenced_by(cms)) {
                Some(Some(held)) => verdict(held),
                _ => Verdict::NotEvaluated,
            },
        };
        out.push(Finding {
            verdict: reference,
            constraint: Constraint::CertificateReference(signer.certificate_reference),
        });
        let info = match signer.certificate_info {
            CertificateInfo::None => None,
            CertificateInfo::SignerOnly => Some(verdict(facts.carries_signer_certificate)),
            // The path is built over the certificates the `SignedData` carries and nothing
            // else, so a path that validated is one the signature carries whole.
            CertificateInfo::FullPath => Some(if path.is_some() {
                Verdict::Met
            } else {
                Verdict::NotEvaluated
            }),
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
    judge_trust_conditions(rules, facts, paths, out);
    judge_issuers(rules, facts, paths, out);
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
    if rules.attribute_algorithms {
        out.push(Finding {
            verdict: Verdict::NotEvaluated,
            constraint: Constraint::CertificateAlgorithms("aaCertAlgorithmConstraints"),
        });
    }
}

/// Clauses 6.7 to 6.9's trust conditions: the signing-certificate and time-stamp conditions as
/// their searches found them, each with its revocation requirement, and the attribute condition
/// named.
fn judge_trust_conditions(
    rules: &Rules<'_>,
    facts: &SignerFacts<'_>,
    paths: Paths<'_>,
    out: &mut Vec<Finding>,
) {
    for condition in &rules.trust_conditions {
        match (*condition, &rules.signer_trust, &rules.timestamp_trust) {
            (SIGNING_CERT_TRUST, Some(trust), _) => {
                trust_path_findings(condition, paths.signer, Some(&trust.revocation), facts, out);
            }
            (TIMESTAMP_TRUST, _, Some(stamp)) => {
                trust_path_findings(
                    condition,
                    paths.authority,
                    stamp.revocation.as_ref(),
                    facts,
                    out,
                );
                authority_name_finding(condition, paths.authority, stamp.names.as_ref(), out);
                for member in &stamp.unprocessed {
                    out.push(Finding {
                        verdict: Verdict::NotEvaluated,
                        constraint: Constraint::ConditionMember { condition, member },
                    });
                }
            }
            _ => out.push(Finding {
                verdict: Verdict::NotEvaluated,
                constraint: Constraint::TrustCondition(condition),
            }),
        }
    }
}

/// A time-stamp trust condition's name constraints, where it states them, judged on the
/// authority's certificate the condition's search found (ADR 2724).
pub(super) fn authority_name_finding(
    condition: &'static str,
    search: Option<&certification_path::Search>,
    names: Option<&crate::trust::names::NameConstraints>,
    out: &mut Vec<Finding>,
) {
    if let Some(names) = names {
        let outcome = certification_path::authority_name(search, names);
        out.push(Finding {
            verdict: outcome.verdict(),
            constraint: Constraint::AuthorityName { condition, outcome },
        });
    }
}

/// One trust condition's findings over the search for its path: the path itself, and each kind
/// of certificate's mandated revocation check answered over it (ADRs 2714, 2720). With no search —
/// a condition stating no trees of its own and inheriting none — the condition is named, and its
/// revocation checks are answered as having no path.
pub(super) fn trust_path_findings(
    condition: &'static str,
    search: Option<&certification_path::Search>,
    revocation: Option<&revocation_checks::Requirement>,
    facts: &SignerFacts<'_>,
    out: &mut Vec<Finding>,
) {
    out.push(match search {
        Some(search) => Finding {
            verdict: search.verdict(),
            constraint: Constraint::TrustPath {
                condition,
                found: search.found.clone(),
                unexamined: search.unexamined,
            },
        },
        None => Finding {
            verdict: Verdict::NotEvaluated,
            constraint: Constraint::TrustCondition(condition),
        },
    });
    let Some(requirement) = revocation else {
        return;
    };
    let path = search.and_then(|search| search.path.as_ref());
    let at = certification_path::instant(facts);
    for check in revocation_checks::checks(requirement, path, &facts.material, at) {
        out.push(Finding {
            verdict: check.verdict(),
            constraint: Constraint::RevocationCheck { condition, check },
        });
    }
    for member in &requirement.unprocessed {
        out.push(Finding {
            verdict: Verdict::NotEvaluated,
            constraint: Constraint::ConditionMember { condition, member },
        });
    }
}

/// Clause 6.10's end-entity and CA certificate-issuer lists, judged along the signer's path, and
/// its time-stamping-authority list, judged on the authority's certificate along the authority's
/// (ADR 2720).
fn judge_issuers(
    rules: &Rules<'_>,
    facts: &SignerFacts<'_>,
    paths: Paths<'_>,
    out: &mut Vec<Finding>,
) {
    if let Some(list) = &rules.authority_certificate_algorithms {
        let authority = paths.authority.and_then(|search| search.path.as_ref());
        let issued = authority.and_then(certification_path::Path::end_entity);
        out.push(issuer_finding(
            TSA_ALGORITHMS,
            list,
            issued.as_ref(),
            authority.is_some(),
        ));
    }
    let path = paths.signer.and_then(|search| search.path.as_ref());
    if let Some(list) = &rules.end_entity_algorithms {
        let issued = facts
            .certificate_signature_algorithm
            .as_ref()
            .map(|algorithm| certification_path::Issued {
                algorithm: algorithm.clone(),
                issuer_key_bits: path
                    .and_then(certification_path::Path::end_entity)
                    .and_then(|issued| issued.issuer_key_bits),
            });
        out.push(issuer_finding(
            END_ENTITY_ALGORITHMS,
            list,
            issued.as_ref(),
            path.is_some(),
        ));
    }
    if let Some(list) = &rules.authority_algorithms {
        let authorities = path.map(certification_path::Path::authorities);
        match authorities.as_deref() {
            Some([]) | None => {
                out.push(issuer_finding(
                    AUTHORITY_ALGORITHMS,
                    list,
                    None,
                    path.is_some(),
                ));
            }
            Some(authorities) => {
                for issued in authorities {
                    out.push(issuer_finding(
                        AUTHORITY_ALGORITHMS,
                        list,
                        Some(issued),
                        true,
                    ));
                }
            }
        }
    }
}

/// One certificate on the path against one of clause 6.10's certificate-issuer lists: its
/// algorithm must be listed, and the issuer's key must be at least the list's minimum for it.
/// With no certificate, the list is met where a path validated and has none to judge, and not
/// evaluated where no path did.
fn issuer_finding(
    member: &'static str,
    list: &[AlgorithmAndLength<'_>],
    issued: Option<&certification_path::Issued>,
    path: bool,
) -> Finding {
    let Some(issued) = issued else {
        return Finding {
            verdict: if path {
                Verdict::Met
            } else {
                Verdict::NotEvaluated
            },
            constraint: Constraint::IssuerAlgorithm {
                member,
                issued: None,
                path,
                listed: false,
                min_key_length: None,
            },
        };
    };
    let entry = list
        .iter()
        .find(|entry| entry.algorithm == issued.algorithm.as_slice());
    let min_key_length = entry.and_then(|entry| entry.min_key_length);
    let verdict = match (entry, min_key_length, issued.issuer_key_bits) {
        (None, _, _) => Verdict::Broken,
        (Some(_), None, _) => Verdict::Met,
        (Some(_), Some(_), None) => Verdict::NotEvaluated,
        (Some(_), Some(minimum), Some(bits)) => {
            if usize::try_from(minimum).is_ok_and(|minimum| bits >= minimum) {
                Verdict::Met
            } else {
                Verdict::Broken
            }
        }
    };
    Finding {
        verdict,
        constraint: Constraint::IssuerAlgorithm {
            member,
            issued: Some((name(&issued.algorithm), issued.issuer_key_bits)),
            path,
            listed: entry.is_some(),
            min_key_length,
        },
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
                    "the policy's requirement that the signing-certificate attribute reference every certificate up to a trust point is {verdict}{}",
                    path_said(self.verdict)
                ),
            },
            Constraint::CertificateInfo(info) => match info {
                CertificateInfo::FullPath => write!(
                    f,
                    "the policy's requirement that the signature carry every certificate up to a trust point is {verdict}{}",
                    path_said(self.verdict)
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
            Constraint::TrustCondition(_)
            | Constraint::TrustPath { .. }
            | Constraint::RevocationCheck { .. }
            | Constraint::ConditionMember { .. }
            | Constraint::AuthorityName { .. }
            | Constraint::IssuerAlgorithm { .. }
            | Constraint::CertificateAlgorithms(_) => self.constraint.path_sentence(f, verdict),
            Constraint::Extension(oid) => write!(
                f,
                "the policy's extension {oid} is {verdict}: its syntax is whatever its identifier names"
            ),
            Constraint::PolicyComponent(constraint) => constraint.sentence(f, verdict),
        }
    }
}

impl Constraint {
    /// The sentences of the constraints on the certification path (ADR 2714).
    fn path_sentence(&self, f: &mut std::fmt::Formatter<'_>, verdict: &str) -> std::fmt::Result {
        match self {
            Self::TrustCondition(condition) if *condition == ATTRIBUTE_TRUST => write!(
                f,
                "the policy's {condition} is {verdict}: it concerns attribute certificates, which this program does not read"
            ),
            Self::TrustCondition(condition) => write!(
                f,
                "the policy's {condition} is {verdict}: it states no trust trees of its own and no rule applies another condition's, so no path was searched for it"
            ),
            Self::TrustPath {
                condition,
                found,
                unexamined,
            } => certification_path::sentence(f, condition, found, *unexamined, verdict),
            Self::RevocationCheck { condition, check } => {
                revocation_checks::sentence(f, condition, check, verdict)
            }
            Self::AuthorityName { condition, outcome } => {
                certification_path::authority_name_sentence(f, condition, outcome, verdict)
            }
            Self::ConditionMember { condition, member } => write!(
                f,
                "the {member} the policy's {condition} states is {verdict}: this program does not evaluate it"
            ),
            Self::IssuerAlgorithm {
                member,
                issued,
                path,
                listed,
                min_key_length,
            } => issuer_sentence(
                f,
                verdict,
                member,
                issued.as_ref(),
                *path,
                *listed,
                *min_key_length,
            ),
            Self::CertificateAlgorithms(member) => write!(
                f,
                "the policy's {member} is {verdict}: it constrains attribute certificates, which this program does not read"
            ),
            _ => Ok(()),
        }
    }
}

/// [`Constraint::IssuerAlgorithm`]'s sentence.
fn issuer_sentence(
    f: &mut std::fmt::Formatter<'_>,
    verdict: &str,
    member: &str,
    issued: Option<&(String, Option<usize>)>,
    path: bool,
    listed: bool,
    min_key_length: Option<u32>,
) -> std::fmt::Result {
    write!(f, "the policy's {member} is {verdict}")?;
    match issued {
                    Some((algorithm, bits)) => {
                        write!(
                            f,
                            " for a certificate issued under {}",
certification_path::issued_words(algorithm, *bits)
                        )?;
                        if !listed {
                            f.write_str(", an algorithm the list does not name")?;
                        } else if let Some(minimum) = min_key_length {
                            write!(f, ", the list's minimum being {minimum} bits")?;
                        }
                        Ok(())
                    }
                    None if path => f.write_str(
                        ": no CA certificate stands between the signer's certificate and the policy's trust point",
                    ),
                    None => f.write_str(
                        ": it constrains the issuers along the certification path, and no path to the policy's own trust points validated",
                    ),
                }
}

/// The clause a full-path requirement's sentence ends with, by its verdict.
const fn path_said(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::NotEvaluated => {
            ": the path is known only once one validates to the policy's own trust points, and none did"
        }
        Verdict::Met | Verdict::Broken => {
            ", over the path that validated to the policy's own trust points"
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
