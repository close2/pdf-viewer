//! The signature policy a `PAdES-E-EPES` signature is made under, read as far as the held texts
//! reach.
//!
//! ISO 32000-2 §12.8.3.4.4 builds its explicit-policy profile out of one attribute and then hands
//! the rules for it to another document — "a signature-policy-identifier shall be present as a
//! signed attribute. The rules from clause 5.2.9 in `CAdES` (ETSI EN 319 122-1) shall apply." Those
//! rules, and the store clause beside them, are what this module reads. ETSI's texts are cited by
//! clause and never quoted: their notice permits no reproduction (ADR 1085).
//!
//! # The three questions this module answers, and the one it does not
//!
//! §12.8.3.4.4 states one obligation on a validator about the policy — that a conforming signature
//! handler "shall enforce signature policy constraints" — and enforcing a constraint means reading
//! the policy document, which is written in a syntax the signature itself names and this project
//! does not hold. What is *not* in that gap, and is done here:
//!
//! - **Which policy.** [`SignaturePolicy::identifier`] is the object identifier the signer signed,
//!   as dotted decimal.
//! - **Whether the signer committed to a particular policy document.** ETSI EN 319 122-1 clause
//!   5.2.9.1 carries a digest beside the identifier and defines an all-zero value of any length,
//!   including an empty one, as the producer saying the digest is not known — a reading it requires
//!   of a validating application rather than permitting. [`PolicyHash`] is those two answers kept
//!   apart.
//! - **Whether the document the file carries is the one the signer committed to.** Clause 5.2.10's
//!   store may hold the policy document itself so that it can be validated with nothing fetched,
//!   and that clause's own note states what the comparison buys: the store is unsigned, so an
//!   alteration of the document in it is caught by the digests failing to agree.
//!   [`SignaturePolicy::binding`] makes that comparison.
//!
//! # Why a mismatch is not called an alteration
//!
//! Clause 5.2.9.1 says that what goes into the digest depends on the technical specification the
//! policy is written under, and clause 5.2.9.2 gives the signer a qualifier for naming that
//! specification precisely because it decides such things. This program hashes the stored octets,
//! which is the input for a policy document stored as it stands; under a specification that
//! prescribes a canonicalisation first, the same document would digest to something else. So a
//! match is decisive — it cannot happen by accident — and a mismatch is reported as
//! [`Binding::DoesNotMatchTheStoredOctets`] carrying the specification the file named, which is a
//! sentence a reader can act on rather than a verdict this program cannot support.
//!
//! ADR 1219 is the argument, and says what stays blocked and on what.

use crate::cms::{Digest, ID_AA_ETS_SIG_POLICY_ID, ID_AA_ETS_SIG_POLICY_STORE, SignedData};
use crate::der::{DerError, OBJECT_IDENTIFIER, OCTET_STRING, SEQUENCE, Value};
use crate::x509::dotted;

/// `id-spq-ets-uri`, `1.2.840.113549.1.9.16.5.1` — ETSI EN 319 122-1 clause 5.2.9.2's qualifier
/// holding a URL a copy of the policy document can be fetched from.
const ID_SPQ_ETS_URI: &[u8] = &[
    0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x10, 0x05, 0x01,
];
/// `id-spq-ets-unotice`, `1.2.840.113549.1.9.16.5.2` — clause 5.2.9.2's qualifier holding a notice
/// the same clause says is meant to be shown whenever the signature is validated.
const ID_SPQ_ETS_UNOTICE: &[u8] = &[
    0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x10, 0x05, 0x02,
];
/// `id-spq-ets-docspec`, `0.4.0.19122.2.1` — clause 5.2.9.2's qualifier naming the technical
/// specification the policy document's syntax is defined by.
const ID_SPQ_ETS_DOCSPEC: &[u8] = &[0x04, 0x00, 0x81, 0x95, 0x32, 0x02, 0x01];

/// `IA5String`, primitive and universal — the type clause 5.2.9.2 gives its URI qualifier.
const IA5_STRING: u8 = 0x16;
/// `UTF8String`, one of the three alternatives clause 5.2.9.2's display text is a choice of.
const UTF8_STRING: u8 = 0x0C;
/// `VisibleString`, the second of them.
const VISIBLE_STRING: u8 = 0x1A;
/// `BMPString`, the third — UTF-16 big-endian, which is why it is decoded rather than copied.
const BMP_STRING: u8 = 0x1E;
/// `INTEGER`, for the notice numbers.
const INTEGER: u8 = 0x02;
/// `NULL`, which is the encoding of the alternative clause 5.2.9.1 forbids outright.
const NULL: u8 = 0x05;

/// What stopped a signature policy from being read.
///
/// A refusal rather than an absence, for the reason [`crate::ess`] states about its own attribute:
/// a caller that cannot read what the signer signed has to say so, because the alternative is a
/// report that looks like a signature made under no policy at all.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PolicyError {
    /// An attribute value is not the structure ETSI EN 319 122-1 clause 5.2.9 or 5.2.10 defines.
    ///
    /// One variant for both attributes because the store is only readable at all beside the
    /// identifier, and a caller that has to refuse has the same thing to say either way.
    #[error("a signature policy attribute is not the structure ETSI EN 319 122-1 defines")]
    Malformed,
    /// The attribute states the implied-policy alternative, which clause 5.2.9.1 forbids.
    ///
    /// Kept apart from [`Self::Malformed`] because it is well-formed and disallowed rather than
    /// unreadable, which is what
    /// [`crate::signature::PadesDeparture::SignaturePolicyImplied`] reports.
    #[error("the signature-policy-identifier attribute states the implied-policy alternative")]
    Implied,
    /// The attribute holds no value, so nothing names a policy.
    #[error("the signature-policy-identifier attribute holds no value")]
    NoValue,
    /// The encoding itself would not read.
    #[error("the signature-policy-identifier attribute would not parse: {0}")]
    Encoding(#[from] DerError),
}

/// Which technical specification the policy document's syntax is defined by.
///
/// ETSI EN 319 122-1 clause 5.2.9.2 makes this a choice of an object identifier or a URI and says
/// what naming it settles — whether the document is human-readable, XML or ASN.1 — and that is
/// exactly the fact this program has to have before the policy's own text means anything. It is
/// therefore reported rather than resolved: the specification is named per signature by the
/// signer, so what blocks enforcement is not one missing document but whichever one a given file
/// points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Specification {
    /// The specification named by an object identifier, as dotted decimal.
    ObjectIdentifier(String),
    /// The specification named by a URI.
    Uri(String),
}

/// One of ETSI EN 319 122-1 clause 5.2.9.2's qualifiers of the policy identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Qualifier {
    /// A URL a copy of the policy document can be obtained from.
    ///
    /// Reported and never fetched: this program makes no network request, and the clause makes
    /// this a convenience for an application that does.
    Uri(String),
    /// A notice clause 5.2.9.2 says is meant to be displayed whenever the signature is validated.
    UserNotice(UserNotice),
    /// The technical specification the policy document's syntax is defined by.
    DocumentSpecification(Specification),
    /// A qualifier outside the three the clause identifies, named by its own object identifier.
    ///
    /// The clause's type is open — the qualifier's syntax is whatever its identifier says it is —
    /// so an unknown one is named rather than guessed at or dropped.
    Other(String),
}

/// ETSI EN 319 122-1 clause 5.2.9.2's user notice, split the way that clause splits it.
///
/// Both members are optional. The clause gives each its own job: the explicit text is the text of
/// the notice to show, and the reference names an organisation and a group of statements by number
/// so that an application holding that organisation's notices file can look the wording up.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UserNotice {
    /// The organisation the numbered statements belong to.
    pub organization: Option<String>,
    /// Which of that organisation's statements the notice refers to.
    pub numbers: Vec<i64>,
    /// The text of the notice itself, where the signer wrote one out.
    pub text: Option<String>,
}

/// What the policy identifier says about the policy document's digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyHash<'a> {
    /// A digest the signer computed over the policy document, under a function this crate has.
    Stated {
        /// The function it was computed with.
        digest: Digest,
        /// The digest itself.
        value: &'a [u8],
    },
    /// A digest under a function this crate does not compute, named by its own identifier.
    ///
    /// Reported rather than approximated, for [`Digest::from_oid`]'s reason: hashing with the
    /// wrong function produces a mismatch, and a mismatch here would read as a substituted policy.
    UnderAnotherFunction {
        /// The algorithm's object identifier as dotted decimal, or its octets in hexadecimal where
        /// they do not decode to one.
        algorithm: String,
        /// The digest itself.
        value: &'a [u8],
    },
    /// The all-zero value ETSI EN 319 122-1 clause 5.2.9.1 defines as *the digest is not known*.
    ///
    /// Any length, an empty one included, which that clause requires a validating application to
    /// accept; a signature stating it has committed to a policy by name and to no particular copy
    /// of its document.
    NotKnown,
}

/// What ETSI EN 319 122-1 clause 5.2.10's store carries, where a file states one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Store<'a> {
    /// The policy document itself, so that it can be validated with nothing fetched.
    Document {
        /// The technical specification its syntax is defined by.
        specification: Specification,
        /// The document's octets as the file holds them.
        octets: &'a [u8],
    },
    /// A URI into a local store the document can be retrieved from.
    ///
    /// The clause's own note distinguishes this from the identifier's URL qualifier: this one
    /// points at a local file. Reported and not opened — nothing in this crate touches a
    /// filesystem.
    LocalUri {
        /// The technical specification its syntax is defined by.
        specification: Specification,
        /// The URI itself.
        uri: String,
    },
}

/// Whether the policy document a file carries is the one the signer committed to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Binding {
    /// The stored document digests to the value the signer signed over it.
    ///
    /// Decisive in this direction and in this direction only: a digest does not agree by accident,
    /// so a match says the store holds the signer's policy whatever specification it is written
    /// under.
    Matches {
        /// The function the comparison was made under.
        digest: Digest,
    },
    /// The stored document's octets digest to something else.
    ///
    /// **Not an alteration on its own.** ETSI EN 319 122-1 clause 5.2.9.1 makes the digest's input
    /// depend on the technical specification the policy is written under, so a specification
    /// prescribing a canonicalisation first would digest the same document differently. The
    /// specification the file named is carried here so that a reader can say which of the two it
    /// is looking at.
    DoesNotMatchTheStoredOctets {
        /// The function the comparison was made under.
        digest: Digest,
        /// The specification the store named for the document's syntax.
        specification: Specification,
    },
    /// The identifier states clause 5.2.9.1's all-zero value, so there is nothing to compare with.
    PolicyHashNotKnown,
    /// The digest is under a function this crate does not compute.
    UnderAnotherFunction {
        /// The algorithm's object identifier as dotted decimal.
        algorithm: String,
    },
    /// No store, or a store holding a URI rather than the document.
    NoStoredDocument,
}

/// The signature policy a signer committed to, as far as the texts this project holds reach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignaturePolicy<'a> {
    /// The policy's object identifier as dotted decimal, or its octets in hexadecimal where they
    /// do not decode to one.
    ///
    /// ETSI EN 319 122-1 clause 5.2.9.1 makes this identify one particular *version* of a policy,
    /// which is why it is the whole of what a report can say about which rules were agreed.
    pub identifier: String,
    /// What the identifier says about the policy document's digest.
    pub hash: PolicyHash<'a>,
    /// Clause 5.2.9.2's qualifiers, in the order the file states them.
    pub qualifiers: Vec<Qualifier>,
    /// Clause 5.2.10's store, where the file carries one.
    pub store: Option<Store<'a>>,
}

impl<'a> SignaturePolicy<'a> {
    /// The policy a signer committed to, read out of a `SignedData`'s attributes.
    ///
    /// `Ok(None)` where the file states no policy identifier at all, which is §12.8.3.4.4's basic
    /// profile and not a fault. The store is read alongside it because the two are one question:
    /// clause 5.2.10's attribute is meaningless without the identifier's digest to check it
    /// against, which is the rule
    /// [`crate::signature::PadesDeparture::PolicyStoreWithoutPolicyDigest`] already reports.
    ///
    /// # Errors
    ///
    /// [`PolicyError`], each variant of which a caller reports rather than treating as an absent
    /// policy; see the module documentation.
    pub fn read(cms: &SignedData<'a>) -> Result<Option<Self>, PolicyError> {
        let Some(attribute) = cms.attribute(ID_AA_ETS_SIG_POLICY_ID) else {
            return Ok(None);
        };
        let Some(value) = attribute.first else {
            return Err(PolicyError::NoValue);
        };
        if value.identifier == NULL {
            return Err(PolicyError::Implied);
        }
        if value.identifier != SEQUENCE {
            return Err(PolicyError::Malformed);
        }
        let mut members = value.children()?;
        let Some(identifier) = members.next_value()? else {
            return Err(PolicyError::Malformed);
        };
        if identifier.identifier != OBJECT_IDENTIFIER {
            return Err(PolicyError::Malformed);
        }
        let Some(hash) = members.next_value()? else {
            return Err(PolicyError::Malformed);
        };
        let hash = policy_hash(&hash)?;
        let qualifiers = match members.next_value()? {
            Some(stated) if stated.identifier == SEQUENCE => read_qualifiers(&stated)?,
            Some(_) => return Err(PolicyError::Malformed),
            None => Vec::new(),
        };
        Ok(Some(Self {
            identifier: name_of(identifier.contents),
            hash,
            qualifiers,
            store: store(cms)?,
        }))
    }

    /// Whether the policy document the file carries is the one the signer committed to.
    ///
    /// ETSI EN 319 122-1 clause 5.2.10's note is the whole argument for making this comparison:
    /// the store is an unsigned attribute, so anybody can replace what is in it, and the digest
    /// the signer put in the signed identifier is what catches that. See [`Binding`] for why the
    /// two outcomes are not symmetrical.
    #[must_use]
    pub fn binding(&self) -> Binding {
        let (specification, octets) = match &self.store {
            Some(Store::Document {
                specification,
                octets,
            }) => (specification, *octets),
            Some(Store::LocalUri { .. }) | None => return Binding::NoStoredDocument,
        };
        match &self.hash {
            PolicyHash::NotKnown => Binding::PolicyHashNotKnown,
            PolicyHash::UnderAnotherFunction { algorithm, .. } => Binding::UnderAnotherFunction {
                algorithm: algorithm.clone(),
            },
            PolicyHash::Stated { digest, value } => {
                let mut hasher = digest.hasher();
                hasher.update(octets);
                if hasher.finish() == *value {
                    Binding::Matches { digest: *digest }
                } else {
                    Binding::DoesNotMatchTheStoredOctets {
                        digest: *digest,
                        specification: specification.clone(),
                    }
                }
            }
        }
    }

    /// The notices clause 5.2.9.2 says are meant to be shown whenever the signature is validated.
    ///
    /// A list because the clause permits a qualifier type to appear more than once, and empty for
    /// a policy that states none — which is most of them.
    #[must_use]
    pub fn notices(&self) -> Vec<&UserNotice> {
        self.qualifiers
            .iter()
            .filter_map(|qualifier| match qualifier {
                Qualifier::UserNotice(notice) => Some(notice),
                _ => None,
            })
            .collect()
    }

    /// The technical specification the signer named for the policy document's syntax.
    ///
    /// The store's is preferred over the identifier's qualifier because clause 5.2.10 makes the
    /// store state the specification of the document it actually holds; where neither is present,
    /// clause 5.2.9.1 leaves the specification to the context, and a reader outside that context
    /// has nothing — which is what `None` says.
    #[must_use]
    pub fn specification(&self) -> Option<&Specification> {
        match &self.store {
            Some(Store::Document { specification, .. } | Store::LocalUri { specification, .. }) => {
                Some(specification)
            }
            None => self
                .qualifiers
                .iter()
                .find_map(|qualifier| match qualifier {
                    Qualifier::DocumentSpecification(specification) => Some(specification),
                    _ => None,
                }),
        }
    }
}

/// ETSI EN 319 122-1 clause 5.2.9.1's digest beside the policy identifier.
///
/// An `AlgorithmIdentifier` and an `OCTET STRING`, with the clause's all-zero value separated out
/// before the algorithm is looked at: a producer saying the digest is not known has said nothing
/// about which function it would have used.
fn policy_hash<'a>(value: &Value<'a>) -> Result<PolicyHash<'a>, PolicyError> {
    if value.identifier != SEQUENCE {
        return Err(PolicyError::Malformed);
    }
    let mut parts = value.children()?;
    let Some(algorithm) = parts.next_value()? else {
        return Err(PolicyError::Malformed);
    };
    if algorithm.identifier != SEQUENCE {
        return Err(PolicyError::Malformed);
    }
    let Some(oid) = algorithm.children()?.next_value()? else {
        return Err(PolicyError::Malformed);
    };
    if oid.identifier != OBJECT_IDENTIFIER {
        return Err(PolicyError::Malformed);
    }
    let Some(octets) = parts.next_value()? else {
        return Err(PolicyError::Malformed);
    };
    if octets.identifier != OCTET_STRING {
        return Err(PolicyError::Malformed);
    }
    if octets.contents.iter().all(|octet| *octet == 0) {
        return Ok(PolicyHash::NotKnown);
    }
    Ok(match Digest::from_oid(oid.contents) {
        Some(digest) => PolicyHash::Stated {
            digest,
            value: octets.contents,
        },
        None => PolicyHash::UnderAnotherFunction {
            algorithm: name_of(oid.contents),
            value: octets.contents,
        },
    })
}

/// Clause 5.2.9.2's `SEQUENCE OF SigPolicyQualifierInfo`, each read by its own identifier.
fn read_qualifiers(value: &Value<'_>) -> Result<Vec<Qualifier>, PolicyError> {
    let mut out = Vec::new();
    let mut each = value.children()?;
    while let Some(info) = each.next_value()? {
        if info.identifier != SEQUENCE {
            return Err(PolicyError::Malformed);
        }
        let mut parts = info.children()?;
        let Some(oid) = parts.next_value()? else {
            return Err(PolicyError::Malformed);
        };
        if oid.identifier != OBJECT_IDENTIFIER {
            return Err(PolicyError::Malformed);
        }
        // The qualifier itself is `OPTIONAL` in the clause's class definition, so a bare
        // identifier is well-formed and says only which kind was meant.
        let qualifier = parts.next_value()?;
        out.push(match (oid.contents, qualifier) {
            (ID_SPQ_ETS_URI, Some(uri)) => Qualifier::Uri(string_of(&uri)?),
            (ID_SPQ_ETS_UNOTICE, Some(notice)) => Qualifier::UserNotice(user_notice(&notice)?),
            (ID_SPQ_ETS_DOCSPEC, Some(spec)) => {
                Qualifier::DocumentSpecification(specification(&spec)?)
            }
            // The class definition makes a qualifier optional in general, and all three of the
            // qualifiers the clause identifies declare one — so a known identifier standing alone
            // is malformed rather than a fourth kind.
            (ID_SPQ_ETS_URI | ID_SPQ_ETS_UNOTICE | ID_SPQ_ETS_DOCSPEC, None) => {
                return Err(PolicyError::Malformed);
            }
            (other, _) => Qualifier::Other(name_of(other)),
        });
    }
    Ok(out)
}

/// Clause 5.2.9.2's user notice: an optional reference and an optional text, told apart by tag.
///
/// Both members are optional and their tags do not overlap — the reference is a `SEQUENCE` and the
/// text one of three string types — so a file stating one of them is read without counting.
fn user_notice(value: &Value<'_>) -> Result<UserNotice, PolicyError> {
    if value.identifier != SEQUENCE {
        return Err(PolicyError::Malformed);
    }
    let mut notice = UserNotice::default();
    let mut members = value.children()?;
    while let Some(member) = members.next_value()? {
        if member.identifier == SEQUENCE {
            let mut parts = member.children()?;
            let Some(organization) = parts.next_value()? else {
                return Err(PolicyError::Malformed);
            };
            notice.organization = Some(string_of(&organization)?);
            let Some(numbers) = parts.next_value()? else {
                return Err(PolicyError::Malformed);
            };
            if numbers.identifier != SEQUENCE {
                return Err(PolicyError::Malformed);
            }
            let mut each = numbers.children()?;
            while let Some(number) = each.next_value()? {
                if number.identifier != INTEGER {
                    return Err(PolicyError::Malformed);
                }
                notice.numbers.push(notice_number(number.contents)?);
            }
        } else {
            notice.text = Some(string_of(&member)?);
        }
    }
    Ok(notice)
}

/// Clause 5.2.9.2's choice of an object identifier or a URI, told apart by tag.
fn specification(value: &Value<'_>) -> Result<Specification, PolicyError> {
    match value.identifier {
        OBJECT_IDENTIFIER => Ok(Specification::ObjectIdentifier(name_of(value.contents))),
        IA5_STRING => Ok(Specification::Uri(string_of(value)?)),
        _ => Err(PolicyError::Malformed),
    }
}

/// Clause 5.2.10's store, where the signature carries one.
fn store<'a>(cms: &SignedData<'a>) -> Result<Option<Store<'a>>, PolicyError> {
    let Some(attribute) = cms.attribute(ID_AA_ETS_SIG_POLICY_STORE) else {
        return Ok(None);
    };
    let Some(value) = attribute.first else {
        return Err(PolicyError::NoValue);
    };
    if value.identifier != SEQUENCE {
        return Err(PolicyError::Malformed);
    }
    let mut members = value.children()?;
    let Some(spec) = members.next_value()? else {
        return Err(PolicyError::Malformed);
    };
    let specification = specification(&spec)?;
    let Some(document) = members.next_value()? else {
        return Err(PolicyError::Malformed);
    };
    Ok(Some(match document.identifier {
        OCTET_STRING => Store::Document {
            specification,
            octets: document.contents,
        },
        IA5_STRING => Store::LocalUri {
            specification,
            uri: string_of(&document)?,
        },
        _ => return Err(PolicyError::Malformed),
    }))
}

/// An object identifier as the dotted decimal a person reads, or its octets in hexadecimal.
///
/// The fallback is what keeps a malformed identifier reportable instead of unnameable: an
/// identifier this crate cannot decode is still the thing the file stated, and a report that
/// dropped it would say a policy had no name.
fn name_of(oid: &[u8]) -> String {
    dotted(oid).unwrap_or_else(|| {
        use std::fmt::Write as _;
        let mut out = String::with_capacity(oid.len().saturating_mul(2));
        for octet in oid {
            // `write!` to a `String` cannot fail — `fmt::Write for String` returns `Ok` always —
            // and the alternative would be an error path no input can reach.
            let _ = write!(out, "{octet:02X}");
        }
        out
    })
}

/// One of X.680's restricted character strings, decoded to text.
///
/// The three clause 5.2.9.2's display text is a choice of, plus the `IA5String` its URI and the
/// store's local URI are. `BMPString` is UTF-16 big-endian and is decoded rather than copied; the
/// others are byte-per-character subsets of Unicode, and a producer that wrote an octet outside
/// its type's repertoire gets that octet back as its Latin-1 character rather than an error,
/// because dropping a URL over one byte would lose the one thing the qualifier is for.
fn string_of(value: &Value<'_>) -> Result<String, PolicyError> {
    match value.identifier {
        UTF8_STRING => Ok(String::from_utf8_lossy(value.contents).into_owned()),
        IA5_STRING | VISIBLE_STRING => Ok(value.contents.iter().map(|o| char::from(*o)).collect()),
        BMP_STRING => {
            if !value.contents.len().is_multiple_of(2) {
                return Err(PolicyError::Malformed);
            }
            let units: Vec<u16> = value
                .contents
                .chunks_exact(2)
                .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
                .collect();
            Ok(String::from_utf16_lossy(&units))
        }
        _ => Err(PolicyError::Malformed),
    }
}

/// One of clause 5.2.9.2's notice numbers, which index an organisation's statements.
///
/// Non-negative by what it is for, so a negative or oversized encoding is a malformed notice
/// rather than a number to carry.
fn notice_number(contents: &[u8]) -> Result<i64, PolicyError> {
    if contents.is_empty() || contents.len() > 8 || contents[0] & 0x80 != 0 {
        return Err(PolicyError::Malformed);
    }
    let mut value: i64 = 0;
    for octet in contents {
        value = value
            .checked_mul(256)
            .and_then(|shifted| shifted.checked_add(i64::from(*octet)))
            .ok_or(PolicyError::Malformed)?;
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{
        Binding, ID_SPQ_ETS_DOCSPEC, ID_SPQ_ETS_UNOTICE, ID_SPQ_ETS_URI, PolicyHash, Qualifier,
        SignaturePolicy, Specification, Store,
    };
    use crate::cms::{Digest, fixtures, signed_data};
    use crate::der::Reader;
    use crate::x509::dotted;

    /// The three qualifier identifiers, read back as the digits ETSI EN 319 122-1 clause 5.2.9.2
    /// assigns them.
    ///
    /// An identifier written as octets is a *claim about a number*, and `x509::dotted` is what
    /// checks it — the same calibration `cms` and `dsa` apply to their own constants.
    #[test]
    fn each_qualifier_identifier_is_the_number_the_clause_assigns() {
        assert_eq!(
            dotted(ID_SPQ_ETS_URI).as_deref(),
            Some("1.2.840.113549.1.9.16.5.1")
        );
        assert_eq!(
            dotted(ID_SPQ_ETS_UNOTICE).as_deref(),
            Some("1.2.840.113549.1.9.16.5.2")
        );
        assert_eq!(
            dotted(ID_SPQ_ETS_DOCSPEC).as_deref(),
            Some("0.4.0.19122.2.1")
        );
    }

    /// `OtherHashAlgAndValue` over one digest and one value, as clause 5.2.9.1 defines it.
    fn hash(digest: Digest, value: &[u8]) -> Vec<u8> {
        fixtures::tagged(
            0x30,
            &[
                fixtures::tagged(0x30, &[fixtures::primitive(0x06, digest.oid())]),
                fixtures::primitive(0x04, value),
            ],
        )
    }

    /// [`super::policy_hash`] over one of those.
    fn read_hash(bytes: &[u8]) -> PolicyHash<'_> {
        let value = Reader::new(bytes)
            .expect("the fixture is a DER value")
            .next_value()
            .expect("it reads")
            .expect("and there is one");
        super::policy_hash(&value).expect("the fixture is the structure the clause defines")
    }

    /// Clause 5.2.9.1's all-zero value, at three of the lengths it admits.
    ///
    /// The clause requires a validating application to read any all-zero value as *the digest is
    /// not known*, and notes that implementations have written them at differing lengths — the
    /// empty one included, which is why it is asserted beside the others.
    #[test]
    fn an_all_zero_digest_of_any_length_says_the_digest_is_not_known() {
        for value in [&[][..], &[0x00][..], &[0x00; 32][..]] {
            let bytes = hash(Digest::Sha256, value);
            assert_eq!(read_hash(&bytes), PolicyHash::NotKnown, "over {value:?}");
        }
    }

    /// One non-zero octet is a digest, which is the boundary that definition draws.
    #[test]
    fn a_digest_with_one_non_zero_octet_is_a_digest() {
        let mut value = [0x00; 32];
        value[31] = 0x01;
        let bytes = hash(Digest::Sha256, &value);
        let PolicyHash::Stated {
            digest,
            value: read,
        } = read_hash(&bytes)
        else {
            panic!("a value with a non-zero octet is a stated digest");
        };
        assert_eq!(digest, Digest::Sha256);
        assert_eq!(read, &value[..]);
    }

    /// A digest function this crate does not compute is named, never approximated.
    #[test]
    fn a_digest_under_another_function_is_named_by_its_own_identifier() {
        // `1.2.3.4`, which names no digest anybody assigns.
        let bytes = fixtures::tagged(
            0x30,
            &[
                fixtures::tagged(0x30, &[fixtures::primitive(0x06, &[0x2A, 0x03, 0x04])]),
                fixtures::primitive(0x04, &[0xAA; 32]),
            ],
        );
        let PolicyHash::UnderAnotherFunction { algorithm, .. } = read_hash(&bytes) else {
            panic!("an unknown function is reported by its identifier");
        };
        assert_eq!(algorithm, "1.2.3.4");
    }

    /// The whole of clause 5.2.9: which policy, under what digest, with every qualifier read.
    #[test]
    fn a_signature_says_which_policy_it_was_made_under_and_what_the_policy_says_to_show() {
        let bytes = fixtures::pades_under_a_policy(
            &[0x11; 32],
            b"the policy document",
            b"the policy document",
        );
        let cms = signed_data(&bytes).expect("the fixture is a SignedData");
        let policy = SignaturePolicy::read(&cms)
            .expect("the fixture is the structure the clause defines")
            .expect("and it states a policy");

        assert_eq!(policy.identifier, "1.2.3.5");
        assert!(matches!(
            policy.hash,
            PolicyHash::Stated {
                digest: Digest::Sha256,
                ..
            }
        ));
        assert_eq!(
            policy.qualifiers.first(),
            Some(&Qualifier::Uri("https://example.invalid/policy".to_owned()))
        );
        let notices = policy.notices();
        let [notice] = notices.as_slice() else {
            panic!("the fixture states one user notice: {notices:?}");
        };
        assert_eq!(notice.organization.as_deref(), Some("Example Authority"));
        assert_eq!(notice.numbers, [7]);
        assert_eq!(
            notice.text.as_deref(),
            Some("this signature is made under a policy")
        );
        assert_eq!(
            policy.specification(),
            Some(&Specification::ObjectIdentifier("1.2.3.6".to_owned())),
            "the store names the specification of the document it actually holds"
        );
    }

    /// Clause 5.2.10's note, which is why the comparison is made at all: the store is unsigned.
    #[test]
    fn a_stored_policy_document_is_bound_to_the_digest_the_signer_signed() {
        let bytes = fixtures::pades_under_a_policy(
            &[0x11; 32],
            b"the policy document",
            b"the policy document",
        );
        let cms = signed_data(&bytes).expect("the fixture is a SignedData");
        let policy = SignaturePolicy::read(&cms)
            .expect("the fixture reads")
            .expect("and states a policy");
        assert!(matches!(policy.store, Some(Store::Document { .. })));
        assert_eq!(
            policy.binding(),
            Binding::Matches {
                digest: Digest::Sha256
            }
        );
    }

    /// The control for the test above (trap 13): one octet of the stored document turned over.
    ///
    /// Nothing else in the fixture moves — the signature, the message digest and the policy
    /// identifier are the same bytes — so a comparison that had stopped being made would show
    /// itself here as a match rather than as a silence.
    #[test]
    fn a_stored_policy_document_that_was_altered_no_longer_matches() {
        let bytes = fixtures::pades_under_a_policy(
            &[0x11; 32],
            b"bhe policy document",
            b"the policy document",
        );
        let cms = signed_data(&bytes).expect("the fixture is a SignedData");
        let policy = SignaturePolicy::read(&cms)
            .expect("the fixture reads")
            .expect("and states a policy");
        assert_eq!(
            policy.binding(),
            Binding::DoesNotMatchTheStoredOctets {
                digest: Digest::Sha256,
                specification: Specification::ObjectIdentifier("1.2.3.6".to_owned()),
            }
        );
    }

    /// A signature under no policy at all is §12.8.3.4.4's basic profile, not a fault.
    #[test]
    fn a_signature_stating_no_policy_reads_as_no_policy() {
        let bytes = fixtures::detached(&[0x11; 32]);
        let cms = signed_data(&bytes).expect("the fixture is a SignedData");
        assert_eq!(SignaturePolicy::read(&cms), Ok(None));
    }
}
