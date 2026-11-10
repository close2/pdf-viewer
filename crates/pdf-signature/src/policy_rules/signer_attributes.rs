//! The presence and the kind of a signature's signer attributes, judged against what a policy
//! states of them: ETSI TR 102 272 clause 6.9's attribute trust condition, and clause 4.26 of
//! ETSI TS 119 172-2 and TS 119 172-3 (ADRs 2786, 2787). ETSI's texts are cited by clause and
//! paraphrased, never quoted (ADR 1085).
//!
//! # Where a signature carries them
//!
//! Two signed attributes carry signer attributes. ETSI EN 319 122-1 clause 5.2.6.1's
//! `signer-attributes-v2` holds up to three members: attributes the signer claims, attributes
//! an attribute authority certified in attribute certificates, and assertions a third party
//! signed. RFC 5126 section 5.11.3's `signer-attributes`, which EN 319 122-1 Annex A.2.3
//! deprecates without forbidding a validator to read it, holds a list of the first two kinds.
//! Both are signed attributes by their own clauses, and both ASN.1 modules are `EXPLICIT TAGS`,
//! so every context tag wraps the type it names. An unsigned copy of either is not read here: it
//! is a departure §12.8.3.4.3 (j) reports, and nothing the signer signed.
//!
//! # What is decided, and what is named
//!
//! Decided without reading an attribute certificate: whether an attribute of a kind the policy
//! accepts is present (TR 102 272 clause 6.9's `attributeMandated`), whether each attribute type
//! a TS 119 172 set of constraints mandates is carried, and whether every signer attribute the set
//! applies to is of a kind it accepts. A claimed attribute is a plain `Attribute`, so its type is
//! read and compared; an attribute certificate or a signed assertion is counted, never opened.
//! Named as not evaluated: whether a certified attribute or a signed assertion is valid (RFC 5755
//! section 5's conditions under the trust trees and the revocation requirement the policy states),
//! and which types it carries where only that would decide a rule; and every constraint on an
//! attribute's *value*, because two values of one attribute type are equal by that type's own
//! matching rule (ITU-T X.501), which this program does not hold per type.

use super::ts_119_172_3::Identifier;
use super::{Constraint, Finding, Verdict};
use crate::cms::{self, SignedData};
use crate::der::{SEQUENCE, Value};
use crate::signature::name;

/// At most this many claimed attributes, attribute certificates and signed assertions are read
/// out of one signature; past it the signature's signer attributes are named unreadable rather
/// than judged on a part of them.
pub const MAX_SIGNER_ATTRIBUTES: usize = 64;

/// The three kinds of signer attribute of EN 319 122-1 clause 5.2.6.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// An attribute the signer claims, uncertified.
    Claimed,
    /// An attribute certified in an attribute certificate.
    Certified,
    /// An assertion a third party signed.
    SignedAssertion,
}

impl Kind {
    const fn words(self) -> &'static str {
        match self {
            Self::Claimed => "a claimed attribute",
            Self::Certified => "a certified attribute",
            Self::SignedAssertion => "a signed assertion",
        }
    }
}

/// What a policy's `HowCertAttribute` accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Accepted {
    /// `claimedAttribute`, in both syntaxes.
    Claimed,
    /// `certifiedAttribtes`, in both syntaxes.
    Certified,
    /// TS 119 172 clause 4.26's signed assertion.
    SignedAssertion,
    /// TR 102 272 clause 6.9's `either`: claimed or certified. That syntax predates signed
    /// assertions and admits none.
    Either,
    /// TS 119 172 clause 4.26's `any`.
    Any,
}

impl Accepted {
    /// Whether an attribute of this kind is of an accepted kind.
    #[must_use]
    pub const fn admits(self, kind: Kind) -> bool {
        matches!(
            (self, kind),
            (Self::Claimed | Self::Either | Self::Any, Kind::Claimed)
                | (Self::Certified | Self::Either | Self::Any, Kind::Certified)
                | (Self::SignedAssertion | Self::Any, Kind::SignedAssertion)
        )
    }

    const fn words(self) -> &'static str {
        match self {
            Self::Claimed => "claimed",
            Self::Certified => "certified",
            Self::SignedAssertion => "a signed assertion",
            Self::Either => "claimed or certified",
            Self::Any => "claimed, certified or a signed assertion",
        }
    }
}

/// The signer attributes a signature carries, as far as their presence and kind say.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Carried<'a> {
    /// The type of each claimed attribute, as the contents octets of its object identifier.
    pub claimed: Vec<&'a [u8]>,
    /// How many attribute certificates, of either `CertifiedAttributesV2` alternative.
    pub certified: usize,
    /// How many signed assertions.
    pub assertions: usize,
}

impl Carried<'_> {
    /// Whether a certified attribute or a signed assertion of an accepted kind is carried: one
    /// whose validity, and whose types, only a reader of it could answer.
    const fn opaque_accepted(&self, accepted: Accepted) -> bool {
        (self.certified > 0 && accepted.admits(Kind::Certified))
            || (self.assertions > 0 && accepted.admits(Kind::SignedAssertion))
    }

    /// The first of the opaque kinds carried that `accepted` refuses.
    const fn opaque_refused(&self, accepted: Accepted) -> Option<Kind> {
        if self.certified > 0 && !accepted.admits(Kind::Certified) {
            Some(Kind::Certified)
        } else if self.assertions > 0 && !accepted.admits(Kind::SignedAssertion) {
            Some(Kind::SignedAssertion)
        } else {
            None
        }
    }
}

/// Why a signature's signer attributes were not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unreadable {
    /// A signed signer-attributes attribute does not hold exactly one value, which both clauses
    /// require.
    NotOneValue,
    /// A value is not the syntax its attribute's clause defines.
    Malformed,
    /// More than [`MAX_SIGNER_ATTRIBUTES`] entries.
    TooMany,
    /// The signature states more attributes than [`crate::cms`] keeps, so a signer-attributes
    /// attribute may be among those not kept.
    AttributesTruncated,
}

impl Unreadable {
    const fn words(self) -> &'static str {
        match self {
            Self::NotOneValue => "a signer-attributes attribute holds other than one value",
            Self::Malformed => "a signer-attributes value is not its clause's syntax",
            Self::TooMany => "they are more than this program reads",
            Self::AttributesTruncated => {
                "the signature states more attributes than this program keeps"
            }
        }
    }
}

/// The signer attributes of the first signer's signed `signer-attributes-v2` and
/// `signer-attributes`, together.
///
/// # Errors
///
/// [`Unreadable`], where a value is not its clause's syntax or the bound is passed.
pub fn carried<'a>(cms: &SignedData<'a>) -> Result<Carried<'a>, Unreadable> {
    if cms.attributes_truncated {
        return Err(Unreadable::AttributesTruncated);
    }
    let mut out = Carried::default();
    for attribute in cms.attributes.iter().filter(|attribute| attribute.signed) {
        let v2 = attribute.kind == cms::ID_AA_ETS_SIGNER_ATTR_V2;
        if !v2 && attribute.kind != cms::ID_AA_ETS_SIGNER_ATTR {
            continue;
        }
        let (1, Some(value)) = (attribute.values, attribute.first) else {
            return Err(Unreadable::NotOneValue);
        };
        if v2 {
            read_v2(&value, &mut out)?;
        } else {
            read_v1(&value, &mut out)?;
        }
    }
    Ok(out)
}

/// Charges one entry against [`MAX_SIGNER_ATTRIBUTES`].
fn charge(out: &Carried<'_>) -> Result<(), Unreadable> {
    let read = out
        .claimed
        .len()
        .saturating_add(out.certified)
        .saturating_add(out.assertions);
    if read < MAX_SIGNER_ATTRIBUTES {
        Ok(())
    } else {
        Err(Unreadable::TooMany)
    }
}

/// The one value an `EXPLICIT` context tag wraps, of this universal identifier.
fn wrapped<'a>(tagged: &Value<'a>, identifier: u8) -> Result<Value<'a>, Unreadable> {
    let mut inner = tagged.children().map_err(|_| Unreadable::Malformed)?;
    match (inner.next_value(), inner.next_value()) {
        (Ok(Some(value)), Ok(None)) if value.identifier == identifier => Ok(value),
        _ => Err(Unreadable::Malformed),
    }
}

/// The entries of a `SEQUENCE OF`.
fn entries<'a>(list: &Value<'a>) -> Result<Vec<Value<'a>>, Unreadable> {
    if list.identifier != SEQUENCE {
        return Err(Unreadable::Malformed);
    }
    let mut reader = list.children().map_err(|_| Unreadable::Malformed)?;
    let mut out = Vec::new();
    while let Some(entry) = reader.next_value().map_err(|_| Unreadable::Malformed)? {
        if out.len() > MAX_SIGNER_ATTRIBUTES {
            return Err(Unreadable::TooMany);
        }
        out.push(entry);
    }
    Ok(out)
}

/// `ClaimedAttributes ::= SEQUENCE OF Attribute`, where `Attribute ::= SEQUENCE { attrType,
/// attrValues SET OF }`: each attribute's type.
fn read_claimed<'a>(list: &Value<'a>, out: &mut Carried<'a>) -> Result<(), Unreadable> {
    for attribute in entries(list)? {
        charge(out)?;
        let kind = (attribute.identifier == SEQUENCE)
            .then(|| attribute.children().ok()?.next_value().ok()?)
            .flatten()
            .and_then(|kind| kind.object_identifier())
            .ok_or(Unreadable::Malformed)?;
        out.claimed.push(kind);
    }
    Ok(())
}

/// EN 319 122-1 clause 5.2.6.1's `SignerAttributeV2 ::= SEQUENCE { claimedAttributes [0]
/// OPTIONAL, certifiedAttributesV2 [1] OPTIONAL, signedAssertions [2] OPTIONAL }`.
fn read_v2<'a>(value: &Value<'a>, out: &mut Carried<'a>) -> Result<(), Unreadable> {
    let mut previous = None;
    for member in entries(value)? {
        let number = (member.identifier & 0xE0 == 0xA0)
            .then_some(member.identifier & 0x1F)
            .filter(|number| *number <= 2 && previous.is_none_or(|last| *number > last))
            .ok_or(Unreadable::Malformed)?;
        previous = Some(number);
        let list = wrapped(&member, SEQUENCE)?;
        match number {
            0 => read_claimed(&list, out)?,
            1 => {
                // `CertifiedAttributesV2 ::= SEQUENCE OF CHOICE { attributeCertificate [0],
                // otherAttributeCertificate [1] }`.
                for certificate in entries(&list)? {
                    charge(out)?;
                    if certificate.identifier != 0xA0 && certificate.identifier != 0xA1 {
                        return Err(Unreadable::Malformed);
                    }
                    wrapped(&certificate, SEQUENCE)?;
                    out.certified = out.certified.saturating_add(1);
                }
            }
            _ => {
                for assertion in entries(&list)? {
                    charge(out)?;
                    if assertion.identifier != SEQUENCE {
                        return Err(Unreadable::Malformed);
                    }
                    out.assertions = out.assertions.saturating_add(1);
                }
            }
        }
    }
    Ok(())
}

/// RFC 5126 section 5.11.3's `SignerAttribute ::= SEQUENCE OF CHOICE { claimedAttributes [0],
/// certifiedAttributes [1] AttributeCertificate }`.
fn read_v1<'a>(value: &Value<'a>, out: &mut Carried<'a>) -> Result<(), Unreadable> {
    for member in entries(value)? {
        match member.identifier {
            0xA0 => read_claimed(&wrapped(&member, SEQUENCE)?, out)?,
            0xA1 => {
                charge(out)?;
                wrapped(&member, SEQUENCE)?;
                out.certified = out.certified.saturating_add(1);
            }
            _ => return Err(Unreadable::Malformed),
        }
    }
    Ok(())
}

/// One attribute type a TS 119 172 set of constraints requires, from clause 4.26's item 4.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mandated<'a> {
    /// The signer attribute identifier: an object identifier, or in TS 119 172-2 a URI that is
    /// not one.
    pub identifier: Identifier<'a>,
    /// How many constraints on the attribute's value it states.
    pub value_constraints: usize,
}

/// TS 119 172-2 and TS 119 172-3 clause 4.26's constraints on one set of signer attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetConstraints<'a> {
    /// The level of assurance: item 1.
    pub accepted: Accepted,
    /// Whether item 2's trust trees are stated.
    pub trust_trees: bool,
    /// Whether item 3's revocation trust is stated.
    pub revocation: bool,
    /// Item 4's signer attribute identifiers.
    pub mandated: Vec<Mandated<'a>>,
}

/// TR 102 272 clause 6.9's `AttributeTrustCondition`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeTrust<'a> {
    /// `attributeMandated`.
    pub mandated: bool,
    /// `howCertAttribute`.
    pub accepted: Accepted,
    /// Whether `attrCertificateTrustTrees` is stated.
    pub trust_trees: bool,
    /// Whether `attrRevReq` is stated.
    pub revocation: bool,
    /// `attributeTypeConstraints`, where stated.
    pub types: Option<Vec<&'a [u8]>>,
    /// The type of each `attributeValueConstraints` entry, where stated.
    pub value_types: Option<Vec<&'a [u8]>>,
}

/// What one finding about signer attributes judged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Check {
    /// The signature's signer attributes were not read.
    Unreadable(Unreadable),
    /// TR 102 272 clause 6.9: an attribute of an accepted kind, within the condition's
    /// constraints, is present.
    Mandated {
        /// The kinds accepted.
        accepted: Accepted,
    },
    /// TS 119 172 clause 4.26 item 4: the identified attribute is carried, of an accepted kind.
    MustBePresent {
        /// The identifier, as a sentence names it.
        identifier: String,
        /// The kinds accepted.
        accepted: Accepted,
    },
    /// TS 119 172 clause 4.26 item 1: every signer attribute the set applies to is of an
    /// accepted kind.
    Level {
        /// The kinds accepted.
        accepted: Accepted,
        /// The kind of a carried attribute it does not accept, where one is found.
        refused: Option<Kind>,
        /// Whether the set applies only to the attribute types it identifies.
        identified: bool,
    },
    /// Certified attributes or signed assertions of an accepted kind, whose validity is not
    /// evaluated.
    Certified {
        /// How many attribute certificates.
        certified: usize,
        /// How many signed assertions.
        assertions: usize,
    },
    /// Constraints on an identified attribute's value, not evaluated.
    Values {
        /// The identifier, as a sentence names it.
        identifier: String,
    },
    /// TR 102 272 clause 6.9's `attributeValueConstraints`, where a claimed attribute is of a
    /// type only they name.
    ConditionValues,
}

impl Check {
    /// The sentence of a finding about signer attributes.
    pub(super) fn sentence(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        verdict: &str,
    ) -> std::fmt::Result {
        match self {
            Self::Unreadable(why) => write!(
                f,
                "the policy's constraints on signer attributes are {verdict}: {}",
                why.words()
            ),
            Self::Mandated { accepted } => write!(
                f,
                "the policy's attribute trust condition, which mandates a signer attribute that is {}, is {verdict}",
                accepted.words()
            ),
            Self::MustBePresent {
                identifier,
                accepted,
            } => write!(
                f,
                "the policy's requirement of signer attribute {identifier}, {}, is {verdict}",
                accepted.words()
            ),
            Self::Level {
                accepted,
                refused,
                identified,
            } => {
                let over = if *identified {
                    "the signer attributes it identifies"
                } else {
                    "every signer attribute"
                };
                match refused {
                    Some(kind) => write!(
                        f,
                        "the policy's level of assurance for {over}, {}, is {verdict}: the signature carries {}",
                        accepted.words(),
                        kind.words()
                    ),
                    None => write!(
                        f,
                        "the policy's level of assurance for {over}, {}, is {verdict}",
                        accepted.words()
                    ),
                }
            }
            Self::Certified {
                certified,
                assertions,
            } => write!(
                f,
                "the policy's conditions on the signature's {certified} attribute certificate(s) and {assertions} signed assertion(s) are {verdict}: this program does not read them, so neither their validity nor the attributes they carry is known"
            ),
            Self::Values { identifier } => write!(
                f,
                "the policy's constraints on the value of signer attribute {identifier} are {verdict}: a value is compared by its type's own matching rule"
            ),
            Self::ConditionValues => write!(
                f,
                "the attribute trust condition's constraints on attribute values are {verdict}: a value is compared by its type's own matching rule"
            ),
        }
    }
}

/// How an identifier is named in a sentence.
fn spelled(identifier: Identifier<'_>) -> String {
    match identifier {
        Identifier::Oid(oid) => name(oid),
        Identifier::Uri(uri) => uri.to_owned(),
    }
}

const fn finding(verdict: Verdict, check: Check) -> Finding {
    Finding {
        verdict,
        constraint: Constraint::SignerAttribute(check),
    }
}

/// TR 102 272 clause 6.9's attribute trust condition, as presence and kind decide it.
///
/// Where `attributeMandated` is true an attribute within the condition's constraints must be
/// present: a claimed one is within them where the kind admits claimed attributes and its type
/// is among `attributeTypeConstraints`, or no constraint is stated; one whose type only
/// `attributeValueConstraints` names is undecided. Clause 6.9 makes `howCertAttribute` say which
/// attributes are considered, not that any other one breaks the signature, so a carried attribute
/// of another kind is passed over rather than judged.
pub(super) fn judge_condition(
    condition: &AttributeTrust<'_>,
    cms: &SignedData<'_>,
    out: &mut Vec<Finding>,
) {
    let carried = match carried(cms) {
        Ok(carried) => carried,
        Err(why) => {
            out.push(finding(Verdict::NotEvaluated, Check::Unreadable(why)));
            return;
        }
    };
    let accepted = condition.accepted;
    let claimed = if accepted.admits(Kind::Claimed) {
        carried.claimed.as_slice()
    } else {
        &[]
    };
    let stated = |list: &Option<Vec<&[u8]>>, kind: &[u8]| {
        list.as_ref().is_some_and(|list| list.contains(&kind))
    };
    let unconstrained = condition.types.is_none() && condition.value_types.is_none();
    let within = claimed
        .iter()
        .any(|kind| unconstrained || stated(&condition.types, kind));
    let by_value = claimed
        .iter()
        .any(|kind| !stated(&condition.types, kind) && stated(&condition.value_types, kind));
    if condition.mandated {
        let verdict = if within {
            Verdict::Met
        } else if by_value || carried.opaque_accepted(accepted) {
            Verdict::NotEvaluated
        } else {
            Verdict::Broken
        };
        out.push(finding(verdict, Check::Mandated { accepted }));
    }
    if by_value {
        out.push(finding(Verdict::NotEvaluated, Check::ConditionValues));
    }
    if carried.certified > 0 && accepted.admits(Kind::Certified) {
        out.push(finding(
            Verdict::NotEvaluated,
            Check::Certified {
                certified: carried.certified,
                assertions: 0,
            },
        ));
    }
}

/// TS 119 172-2 and TS 119 172-3 clause 4.26's constraints on one set of signer attributes.
///
/// The clause makes a signature carry every attribute a set identifies, and every signer
/// attribute meet the set's requirements; a set that identifies none applies them to every signer
/// attribute, so one that identifies some applies them to those. Every set of a rule applies, so
/// each is judged on its own and any one broken breaks the rule (ADR 2787).
pub(super) fn judge_set(set: &SetConstraints<'_>, cms: &SignedData<'_>, out: &mut Vec<Finding>) {
    let carried = match carried(cms) {
        Ok(carried) => carried,
        Err(why) => {
            out.push(finding(Verdict::NotEvaluated, Check::Unreadable(why)));
            return;
        }
    };
    let accepted = set.accepted;
    let names_types = !set.mandated.is_empty();
    let in_scope = |kind: &&[u8]| {
        !names_types
            || set
                .mandated
                .iter()
                .any(|mandated| mandated.identifier.is(kind))
    };
    let claimed_refused = !accepted.admits(Kind::Claimed) && carried.claimed.iter().any(in_scope);
    let opaque_refused = carried.opaque_refused(accepted);
    // An attribute certificate or a signed assertion of a refused kind breaks a set that applies
    // to every signer attribute; under a set that identifies types, it breaks the set only where
    // it carries one of them, which only a reader of it could say.
    let (verdict, refused) = if claimed_refused {
        (Verdict::Broken, Some(Kind::Claimed))
    } else {
        match opaque_refused {
            Some(kind) if !names_types => (Verdict::Broken, Some(kind)),
            Some(kind) => (Verdict::NotEvaluated, Some(kind)),
            None => (Verdict::Met, None),
        }
    };
    out.push(finding(
        verdict,
        Check::Level {
            accepted,
            refused,
            identified: names_types,
        },
    ));
    for mandated in &set.mandated {
        let claimed = accepted.admits(Kind::Claimed)
            && carried
                .claimed
                .iter()
                .any(|kind| mandated.identifier.is(kind));
        let verdict = if claimed {
            Verdict::Met
        } else if carried.opaque_accepted(accepted) {
            Verdict::NotEvaluated
        } else {
            Verdict::Broken
        };
        let identifier = spelled(mandated.identifier);
        if mandated.value_constraints > 0 && verdict != Verdict::Broken {
            out.push(finding(
                Verdict::NotEvaluated,
                Check::Values {
                    identifier: identifier.clone(),
                },
            ));
        }
        out.push(finding(
            verdict,
            Check::MustBePresent {
                identifier,
                accepted,
            },
        ));
    }
    if carried.opaque_accepted(accepted) {
        out.push(finding(
            Verdict::NotEvaluated,
            Check::Certified {
                certified: if accepted.admits(Kind::Certified) {
                    carried.certified
                } else {
                    0
                },
                assertions: if accepted.admits(Kind::SignedAssertion) {
                    carried.assertions
                } else {
                    0
                },
            },
        ));
    }
}

#[cfg(test)]
pub(super) mod tests;
