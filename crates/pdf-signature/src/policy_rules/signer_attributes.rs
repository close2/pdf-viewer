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
//! Decided: whether an attribute of a kind the policy accepts is present (TR 102 272 clause 6.9's
//! `attributeMandated`), whether each attribute type a TS 119 172 set of constraints mandates is
//! carried, and whether every signer attribute the set applies to is of a kind it accepts. A
//! claimed attribute is a plain `Attribute`, so its type is read and compared. An attribute
//! certificate is read by [`crate::attribute_certificate`] for the types it carries, and judged
//! by [`certified`] under RFC 5755 section 5 and the trust trees the policy states (ADR 2797); it
//! certifies an attribute only where it is valid. Named as not evaluated: a signed assertion,
//! which is counted and never opened; an attribute certificate's own revocation where the policy
//! requires it; and every constraint on an attribute's *value*, because two values of one
//! attribute type are equal by that type's own matching rule (ITU-T X.501), which this program
//! does not hold per type.

use super::certification_path::{Search, TrustTrees};
use super::ts_119_172_3::Identifier;
use super::{Constraint, Finding, SignerFacts, Verdict};
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
    /// Each attribute certificate, of either `CertifiedAttributesV2` alternative, as the
    /// `AttributeCertificate` its tag wraps.
    pub certified: Vec<Value<'a>>,
    /// How many signed assertions.
    pub assertions: usize,
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
        .saturating_add(out.certified.len())
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
                    let certificate = wrapped(&certificate, SEQUENCE)?;
                    out.certified.push(certificate);
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
                let certificate = wrapped(&member, SEQUENCE)?;
                out.certified.push(certificate);
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
    /// Item 2's trust trees, where stated.
    pub trust_trees: Option<TrustTrees<'a>>,
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
    /// `attrCertificateTrustTrees`, where stated.
    pub trust_trees: Option<TrustTrees<'a>>,
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
    /// One attribute certificate of an accepted kind, under RFC 5755 section 5 and the policy's
    /// trust trees.
    Certificate {
        /// Its place among the signature's attribute certificates, from one.
        ordinal: usize,
        /// What section 5 found.
        outcome: certified::Outcome,
    },
    /// Signed assertions of an accepted kind, whose validity is not evaluated.
    Assertions {
        /// How many.
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
            Self::Certificate { ordinal, outcome } => {
                write!(
                    f,
                    "the policy's conditions on the signature's attribute certificate {ordinal} are {verdict}: "
                )?;
                outcome.words(f)
            }
            Self::Assertions { assertions } => write!(
                f,
                "the policy's conditions on the signature's {assertions} signed assertion(s) are {verdict}: this program does not read them, so neither their validity nor the attributes they carry is known"
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

/// What the attribute judgements read beyond the policy and the `SignedData`.
#[derive(Debug, Clone, Copy)]
pub(super) struct Context<'c> {
    /// The signer's facts, for the instant evaluated at.
    pub facts: &'c SignerFacts<'c>,
    /// The search of the signer's path under the signing-certificate condition, where made.
    pub signer_path: Option<&'c Search>,
    /// The signing-certificate condition's trust trees, where stated.
    pub signer_trees: Option<&'c TrustTrees<'c>>,
}

/// Each attribute certificate, judged under `trees` and `revocation`.
fn judge_certificates<'a>(
    carried: &Carried<'a>,
    cms: &SignedData<'a>,
    context: Context<'_>,
    trees: Option<&TrustTrees<'_>>,
    revocation: bool,
) -> Vec<certified::Judged<'a>> {
    let conditions = certified::Conditions {
        trees,
        signer_path: context.signer_path,
        at: super::certification_path::instant(context.facts),
        revocation,
    };
    carried
        .certified
        .iter()
        .map(|certificate| certified::judge(*certificate, cms, conditions))
        .collect()
}

/// One finding per attribute certificate in `judged` that `verdict` gives a verdict.
fn certificate_findings(
    judged: &[certified::Judged<'_>],
    verdict: impl Fn(&certified::Judged<'_>) -> Option<Verdict>,
    out: &mut Vec<Finding>,
) {
    for (index, certificate) in judged.iter().enumerate() {
        if let Some(verdict) = verdict(certificate) {
            out.push(finding(
                verdict,
                Check::Certificate {
                    ordinal: index.saturating_add(1),
                    outcome: certificate.outcome.clone(),
                },
            ));
        }
    }
}

/// TR 102 272 clause 6.9's attribute trust condition.
///
/// Where `attributeMandated` is true an attribute within the condition's constraints must be
/// present: a claimed one, where the kind admits claimed attributes, or one a valid attribute
/// certificate certifies, where it admits certified ones; within them where its type is among
/// `attributeTypeConstraints` or no constraint is stated, and undecided where only
/// `attributeValueConstraints` names its type. Clause 6.9 makes `howCertAttribute` say which
/// attributes are considered, so a carried attribute of another kind is passed over. It makes
/// `attrCertificateTrustTrees` the path conditions for any attribute certificate, the signing
/// certificate's where it states none, so each attribute certificate of an admitted kind is judged
/// under them, and one that fails them breaks the condition (ADR 2797).
pub(super) fn judge_condition(
    condition: &AttributeTrust<'_>,
    cms: &SignedData<'_>,
    context: Context<'_>,
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
    let judged = if accepted.admits(Kind::Certified) {
        judge_certificates(
            &carried,
            cms,
            context,
            condition.trust_trees.as_ref().or(context.signer_trees),
            condition.revocation,
        )
    } else {
        Vec::new()
    };
    let stated = |list: &Option<Vec<&[u8]>>, kind: &[u8]| {
        list.as_ref().is_some_and(|list| list.contains(&kind))
    };
    let unconstrained = condition.types.is_none() && condition.value_types.is_none();
    let within = |kind: &[u8]| unconstrained || stated(&condition.types, kind);
    let by_value =
        |kind: &[u8]| !stated(&condition.types, kind) && stated(&condition.value_types, kind);
    let claimed_by_value = claimed.iter().any(|kind| by_value(kind));
    let certified_by_value = judged.iter().any(|judged| judged.certifies(by_value));
    if condition.mandated {
        let verdict = if claimed.iter().any(|kind| within(kind))
            || judged.iter().any(|judged| judged.certifies(within))
        {
            Verdict::Met
        } else if claimed_by_value
            || certified_by_value
            || judged
                .iter()
                .any(|judged| judged.might_certify(|kind| within(kind) || by_value(kind)))
        {
            Verdict::NotEvaluated
        } else {
            Verdict::Broken
        };
        out.push(finding(verdict, Check::Mandated { accepted }));
    }
    if claimed_by_value || certified_by_value {
        out.push(finding(Verdict::NotEvaluated, Check::ConditionValues));
    }
    certificate_findings(&judged, |judged| Some(judged.outcome.verdict()), out);
}

/// TS 119 172-2 and TS 119 172-3 clause 4.26's constraints on one set of signer attributes.
///
/// The clause makes a signature carry every attribute a set identifies, and every signer
/// attribute meet the set's requirements; a set that identifies none applies them to every signer
/// attribute, so one that identifies some applies them to those. Every set of a rule applies, so
/// each is judged on its own and any one broken breaks the rule (ADR 2787). An attribute
/// certificate is read for the types it carries whatever its kind, and one of an accepted kind
/// the set applies to is judged under the set's trust trees and revocation trust (ADR 2797).
pub(super) fn judge_set(
    set: &SetConstraints<'_>,
    cms: &SignedData<'_>,
    context: Context<'_>,
    out: &mut Vec<Finding>,
) {
    let carried = match carried(cms) {
        Ok(carried) => carried,
        Err(why) => {
            out.push(finding(Verdict::NotEvaluated, Check::Unreadable(why)));
            return;
        }
    };
    let accepted = set.accepted;
    let names_types = !set.mandated.is_empty();
    let in_scope = |kind: &[u8]| {
        !names_types
            || set
                .mandated
                .iter()
                .any(|mandated| mandated.identifier.is(kind))
    };
    let (verdict, refused) = level(&carried, accepted, names_types, &in_scope);
    out.push(finding(
        verdict,
        Check::Level {
            accepted,
            refused,
            identified: names_types,
        },
    ));
    let judged = if accepted.admits(Kind::Certified) {
        judge_certificates(
            &carried,
            cms,
            context,
            set.trust_trees.as_ref(),
            set.revocation,
        )
    } else {
        Vec::new()
    };
    let assertions = if accepted.admits(Kind::SignedAssertion) {
        carried.assertions
    } else {
        0
    };
    for mandated in &set.mandated {
        let is = |kind: &[u8]| mandated.identifier.is(kind);
        let claimed = accepted.admits(Kind::Claimed) && carried.claimed.iter().any(|kind| is(kind));
        let verdict = if claimed || judged.iter().any(|judged| judged.certifies(is)) {
            Verdict::Met
        } else if assertions > 0 || judged.iter().any(|judged| judged.might_certify(is)) {
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
    // A set that identifies types applies to a certificate that carries one; one that does not
    // read might, so its conditions are undecided rather than broken.
    certificate_findings(
        &judged,
        |judged| match &judged.types {
            Some(types) if names_types && !types.iter().any(|kind| in_scope(kind)) => None,
            None if names_types => Some(Verdict::NotEvaluated),
            _ => Some(judged.outcome.verdict()),
        },
        out,
    );
    if assertions > 0 {
        out.push(finding(
            Verdict::NotEvaluated,
            Check::Assertions { assertions },
        ));
    }
}

/// Clause 4.26.1 item 1's level over the signer attributes a set applies to: the verdict, and the
/// kind of a carried attribute below it.
fn level(
    carried: &Carried<'_>,
    accepted: Accepted,
    names_types: bool,
    in_scope: &impl Fn(&[u8]) -> bool,
) -> (Verdict, Option<Kind>) {
    let claimed_refused =
        !accepted.admits(Kind::Claimed) && carried.claimed.iter().any(|kind| in_scope(kind));
    // An attribute certificate of a refused kind breaks the set where it carries a type the set
    // applies to; one that does not read might, so it is undecided. A signed assertion is not
    // read, so under a set that identifies types it is undecided too.
    let certified_refused = !carried.certified.is_empty() && !accepted.admits(Kind::Certified);
    let refused_types: Vec<Option<Vec<&[u8]>>> = if certified_refused {
        carried
            .certified
            .iter()
            .map(|certificate| certified::types_of(*certificate))
            .collect()
    } else {
        Vec::new()
    };
    let refused_carries = refused_types.iter().any(|types| {
        types
            .as_ref()
            .is_some_and(|types| types.iter().any(|kind| in_scope(kind)))
    });
    let refused_unread = refused_types.iter().any(Option::is_none);
    let assertions_refused = carried.assertions > 0 && !accepted.admits(Kind::SignedAssertion);
    if claimed_refused {
        (Verdict::Broken, Some(Kind::Claimed))
    } else if refused_carries {
        (Verdict::Broken, Some(Kind::Certified))
    } else if assertions_refused && !names_types {
        (Verdict::Broken, Some(Kind::SignedAssertion))
    } else if refused_unread {
        (Verdict::NotEvaluated, Some(Kind::Certified))
    } else if assertions_refused {
        (Verdict::NotEvaluated, Some(Kind::SignedAssertion))
    } else {
        (Verdict::Met, None)
    }
}

pub mod certified;

#[cfg(test)]
pub(super) mod tests;
