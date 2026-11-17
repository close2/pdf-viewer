//! ETSI EN 319 102-1 clause 5.2.8.4.2.7's validation of a signed assertion a signature carries,
//! under the constraints a policy's set of signer-attribute constraints states for it (ADR 2922).
//!
//! The clause asks a validator, where the constraints address signed assertions, to validate each
//! one and to check it against the constraints, and makes a failed check the constraints' failure.
//! EN 319 122-1 clause 5.2.6.3 defines one form, `id-ets-signedSAML`, the octets of a signed SAML
//! 2.0 assertion; any other identifier is a form no text defines, so its validity is undecided and
//! said to be. A SAML assertion is valid here where, in this order:
//!
//! 1. it reads as SAML 2.0 core section 2.3.3's `<Assertion>` ([`crate::saml::read`]);
//! 2. it is signed — EN 319 122-1 clause 5.2.6.1 requires every signed assertion to be signed by a
//!    third party — and section 5.4's validation of that signature passes;
//! 3. the certificate whose key verified it validates along a path to the set's trust trees at the
//!    instant evaluated at, which TS 119 172-2 clause 4.26 item 2 states for signed assertions and
//!    EN 319 102-1 clause 5.2.8.4.2.7 item 1 validates by ITU-T X.509's rules;
//! 4. section 2.5.1's conditions are valid at that instant, which section 2.5.1.1 requires of any
//!    assertion a relying party accepts; and
//! 5. where the set states a revocation requirement (TS 119 172-2 clause 4.26 item 3), the path's
//!    certificates answer it.
//!
//! The first failure decides, else the first undecided. SAML core section 2.3.3 defines no relation
//! between an assertion's subject and anything outside it, and EN 319 122-1 none between the subject
//! and the signer, so the subject is shown and never compared.

use super::SignedAssertion;
use super::certified::Conditions;
use crate::cms::SignedData;
use crate::policy_rules::certification_path::{self, Found};
use crate::policy_rules::revocation_checks::{self, KindCheck};
use crate::policy_rules::{ATTRIBUTE_TRUST, Verdict};
use crate::saml::{self, Attribute, Saml, SignatureCheck};
use crate::xml_signature::XmlSignatureError;

/// Why a signed assertion is not valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Invalid {
    /// It does not read as a SAML assertion.
    Unreadable(saml::Unreadable),
    /// It carries no signature.
    Unsigned,
    /// Its signature does not verify, or is of a shape SAML 2.0 core section 5.4 forbids.
    Signature(XmlSignatureError),
    /// Its signer's path to the trust trees is refused. Boxed because a search's finding is many
    /// times the size of every other variant.
    Path(Box<Found>, usize),
    /// The instant is outside its conditions' interval (SAML 2.0 core section 2.5.1.2).
    Outside,
    /// The policy's revocation check found a certificate of its signer's path revoked.
    Revoked(Box<KindCheck>),
}

/// Why a signed assertion's validity is not decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Undecided {
    /// Its identifier, dotted, names a form no text defines.
    Form(String),
    /// `signedAssertion` is absent, so there is nothing to validate.
    Absent,
    /// Its signature could not be verified here: no key carried, or an algorithm not computed.
    Signature(XmlSignatureError),
    /// The set of constraints states no trust trees for signed assertions.
    NoTrustTrees,
    /// Its signer's path is not decided. Boxed as [`Invalid::Path`] is.
    Path(Box<Found>, usize),
    /// No instant was given to evaluate at.
    NoInstant,
    /// A condition cannot be evaluated; which (SAML 2.0 core section 2.5.1.1 rule 3).
    Conditions(&'static str),
    /// The policy's revocation check is not answered for a certificate of the signer's path.
    Revocation(Box<KindCheck>),
}

/// What clause 5.2.8.4.2.7 found of one signed assertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Every check passes.
    Valid,
    /// A check fails; the first in this module's order.
    Invalid(Invalid),
    /// No check fails, and one is not decided; the first in this module's order.
    Undecided(Undecided),
}

impl Outcome {
    /// The verdict on the policy's conditions on this assertion.
    #[must_use]
    pub const fn verdict(&self) -> Verdict {
        match self {
            Self::Valid => Verdict::Met,
            Self::Invalid(_) => Verdict::Broken,
            Self::Undecided(_) => Verdict::NotEvaluated,
        }
    }

    /// The part of a sentence that says why.
    pub(super) fn words(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Valid => f.write_str(
                "its SAML signature verifies, its signer's path validates to the set's trust trees, and its conditions hold at the instant evaluated at (ETSI EN 319 102-1 clause 5.2.8.4.2.7); its subject is not compared with the signer, which no text defines",
            ),
            Self::Invalid(why) => match why {
                Invalid::Unreadable(why) => write!(f, "it is no SAML 2.0 assertion: {why}"),
                Invalid::Unsigned => f.write_str(
                    "it carries no signature, where ETSI EN 319 122-1 clause 5.2.6.1 requires a third party's",
                ),
                Invalid::Signature(why) => write!(
                    f,
                    "its signature is invalid, so SAML 2.0 core section 2.3.3 forbids relying on it: {why}"
                ),
                Invalid::Path(found, unexamined) => {
                    certification_path::path_words(f, ATTRIBUTE_TRUST, found, *unexamined)
                }
                Invalid::Outside => f.write_str(
                    "the instant evaluated at is outside its conditions' interval (SAML 2.0 core section 2.5.1.2)",
                ),
                Invalid::Revoked(check) => {
                    revocation_checks::sentence(f, ATTRIBUTE_TRUST, check, Verdict::Broken.word())
                }
            },
            Self::Undecided(why) => match why {
                Undecided::Form(identifier) => write!(
                    f,
                    "its form {identifier} is not one ETSI EN 319 122-1 defines, so nothing says how to validate it"
                ),
                Undecided::Absent => f.write_str("it carries no assertion to validate"),
                Undecided::Signature(why) => {
                    write!(f, "its signature is unverified, not valid: {why}")
                }
                Undecided::NoTrustTrees => f.write_str(
                    "the set states no trust trees for signed assertions, so its signer is trusted by nothing",
                ),
                Undecided::Path(found, unexamined) => {
                    certification_path::path_words(f, ATTRIBUTE_TRUST, found, *unexamined)
                }
                Undecided::NoInstant => f.write_str("no instant was given to evaluate it at"),
                Undecided::Conditions(which) => write!(
                    f,
                    "its conditions are indeterminate (SAML 2.0 core section 2.5.1.1): {which}"
                ),
                Undecided::Revocation(check) => {
                    revocation_checks::sentence(f, ATTRIBUTE_TRUST, check, Verdict::NotEvaluated.word())
                }
            },
        }
    }
}

/// One signed assertion, judged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Judged {
    /// The attributes it carries; `None` where it was not read, or its signature is invalid and
    /// SAML 2.0 core section 2.3.3 forbids relying on them.
    pub attributes: Option<Vec<Attribute>>,
    /// What clause 5.2.8.4.2.7 found.
    pub outcome: Outcome,
}

impl Judged {
    /// Whether it may carry an attribute `wanted` accepts: one of its attributes is, one is named in
    /// a way SAML 2.0 core section 8.2.2 leaves to the application, or it was not read.
    pub(super) fn may_carry(&self, wanted: impl Fn(&Attribute) -> Option<bool>) -> bool {
        self.attributes
            .as_ref()
            .is_none_or(|attributes| attributes.iter().any(|one| wanted(one) != Some(false)))
    }

    /// Whether it is valid and certainly carries an attribute `wanted` accepts.
    pub(super) fn asserts(&self, wanted: impl Fn(&Attribute) -> Option<bool>) -> bool {
        self.outcome == Outcome::Valid
            && self
                .attributes
                .as_ref()
                .is_some_and(|attributes| attributes.iter().any(|one| wanted(one) == Some(true)))
    }
}

/// Clause 5.2.8.4.2.7 over `assertion`, one of `cms`'s signed assertions.
pub(super) fn judge(
    assertion: &SignedAssertion<'_>,
    cms: &SignedData<'_>,
    conditions: Conditions<'_>,
) -> Judged {
    let undecided = |why| Judged {
        attributes: None,
        outcome: Outcome::Undecided(why),
    };
    if assertion.identifier != saml::ID_ETS_SIGNED_SAML {
        return undecided(Undecided::Form(
            crate::x509::dotted(assertion.identifier)
                .unwrap_or_else(|| "an unreadable object identifier".to_owned()),
        ));
    }
    let Some(octets) = assertion.saml_octets() else {
        return match assertion.assertion {
            None => undecided(Undecided::Absent),
            Some(_) => Judged {
                attributes: None,
                outcome: Outcome::Invalid(Invalid::Unreadable(saml::Unreadable::Malformed(
                    "SignedSAMLAssertion OCTET STRING",
                ))),
            },
        };
    };
    let read = Saml::of(octets);
    let relied_on = read.assertion.is_ok() && !read.signature.is_invalid();
    Judged {
        attributes: relied_on.then(|| read.attributes_relied_on().to_vec()),
        outcome: outcome(&read, cms, conditions),
    }
}

/// Every check, in this module's order; the first failure decides, else the first undecided.
fn outcome(read: &Saml, cms: &SignedData<'_>, conditions: Conditions<'_>) -> Outcome {
    let assertion = match &read.assertion {
        Ok(assertion) => assertion,
        Err(why) => return Outcome::Invalid(Invalid::Unreadable(why.clone())),
    };
    let signer = match &read.signature {
        SignatureCheck::Unsigned => return Outcome::Invalid(Invalid::Unsigned),
        SignatureCheck::Failed(why) if read.signature.is_invalid() => {
            return Outcome::Invalid(Invalid::Signature(why.clone()));
        }
        SignatureCheck::Failed(why) => {
            return Outcome::Undecided(Undecided::Signature(why.clone()));
        }
        SignatureCheck::Verified(signer) => signer,
    };
    let mut undecided = None;
    let path = match path(signer, cms, conditions) {
        Ok(path) => path,
        Err(Ok(invalid)) => return Outcome::Invalid(invalid),
        Err(Err(why)) => {
            undecided.get_or_insert(why);
            None
        }
    };
    match conditions.at.map(|at| assertion.conditions_at(at)) {
        None => {
            undecided.get_or_insert(Undecided::NoInstant);
        }
        Some(saml::Conditions::Invalid) => return Outcome::Invalid(Invalid::Outside),
        Some(saml::Conditions::Indeterminate(which)) => {
            undecided.get_or_insert(Undecided::Conditions(which));
        }
        Some(saml::Conditions::Valid) => {}
    }
    if let Some(requirement) = conditions.revocation {
        let checks = revocation_checks::checks(
            requirement,
            path.as_ref(),
            conditions.material,
            conditions.at,
            conditions.timing,
        );
        for check in checks {
            match check.verdict() {
                Verdict::Met => {}
                Verdict::Broken => return Outcome::Invalid(Invalid::Revoked(Box::new(check))),
                Verdict::NotEvaluated => {
                    undecided.get_or_insert(Undecided::Revocation(Box::new(check)));
                }
            }
        }
    }
    undecided.map_or(Outcome::Valid, Outcome::Undecided)
}

/// Item 3: the signer's path to the set's trust trees, handed back where it validated.
fn path(
    signer: &[u8],
    cms: &SignedData<'_>,
    conditions: Conditions<'_>,
) -> Result<Option<certification_path::Path>, Result<Invalid, Undecided>> {
    let trees = conditions.trees.ok_or(Err(Undecided::NoTrustTrees))?;
    let certificate = crate::der::Reader::new(signer)
        .ok()
        .and_then(|mut reader| reader.next_value().ok().flatten())
        .ok_or(Ok(Invalid::Signature(XmlSignatureError::Malformed(
            "X509Certificate",
        ))))?;
    let search = certification_path::search_certificate(
        trees,
        cms,
        certificate,
        conditions.at,
        conditions.lists,
    );
    match search.verdict() {
        Verdict::Met => Ok(search.path),
        Verdict::Broken => Err(Ok(Invalid::Path(Box::new(search.found), search.unexamined))),
        Verdict::NotEvaluated => Err(Err(Undecided::Path(
            Box::new(search.found),
            search.unexamined,
        ))),
    }
}
