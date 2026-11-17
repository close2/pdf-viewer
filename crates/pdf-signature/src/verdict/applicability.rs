//! ETSI TS 119 172-4 clause 4.5's applicability rules checking report: what REQ-4.5-01 says the
//! report on a signature checked against EU trusted lists holds beside clause 4.4.2's level
//! ([`super::level`]), and REQ-4.2-03 d) and e)'s cryptographic information.
//!
//! # What each item is, and where it comes from
//!
//! - **a)**, the scope of the check, is a text the clause prints and requires; [`Scope`] names it
//!   and carries the clause's heading and sentence as printed ([`Scope::heading`],
//!   [`Scope::text`]). They are the only ETSI words this tree carries as written: the owner read
//!   the requirement to print them as the permission to (`doc/questions/A855`, ADR 2897).
//! - **b)**, the signer as the signing certificate represents it: every attribute of its
//!   `subject` field and, where present, every name of its `subjectAltName` extension
//!   ([`Signer`]).
//! - **c)**, a pseudonym: the clause asks that its use be indicated where one was used at the best
//!   signature time. The certificate used at that time is the signing certificate, and X.520's
//!   `pseudonym` attribute (`2.5.4.65`) in its subject is the use ([`Signer::pseudonym`]).
//! - **d) ii)**, the timing information, each where it applies: the claimed signing time (the
//!   `signingTime` signed attribute, and §12.8.1's `/M`), the document timestamps over this
//!   signature, its own signature timestamp, the revocation of the signer's certificate, the issue
//!   and next-update times of the revocation information about it, and the best signature time
//!   ([`Timing`]).
//! - **d) iii)**, each time's evidential relevance and assurance: a claim is a claim, a token's time
//!   is established or not, and a token carries its policy identifier and accuracy. Whether a
//!   token is an EU qualified time stamp (item 3)) is TS 119 615 clause 4.7's determination — the
//!   clause's NOTE 3 names clause 4.6, the issuer's determination clause 4.7 runs twice — over the
//!   host's lists at the instant the token's authority was asked about and at its `genTime`
//!   ([`EuQualified`]). Clause 4.7.1 determines a status only for a token that is valid, so a
//!   token whose time was not established is said as not determined for that reason.
//! - **d) i)**, the absence of the token's authority from a Member State's list as a service of
//!   type `TSA/QTST` or `TSA`: TS 119 615 clause 4.3, which the clause's NOTE 2 points at, run for
//!   both types at the token's `genTime` ([`Assurance::Token`]'s `listed`, empty for the absence).
//! - **f)**, every attribute of the signer, signed or unsigned, named where this tree knows its
//!   identifier, and a `commitment-type-indication`'s commitment named, which the clause's NOTE 5
//!   points at ([`SignatureAttribute`]). A signed `signer-attributes-v2`'s SAML assertions are
//!   shown with it, read and their signatures checked ([`CarriedAssertion`]): ETSI EN 319 102-1
//!   clause 5.2.8.4.2.7 makes a signed assertion no rule constrains available for the driving
//!   application to decide on, and SAML 2.0 core section 2.3.3 forbids relying on the contents of
//!   one whose signature is invalid (ADR 2922).
//! - **h)** and **REQ-4.2-03 d)**, the cryptographic suite the signature was made with: the digest,
//!   the signature algorithm and the key's size ([`Cryptography::Suite`]). The clause also asks for
//!   potential security issues against either national rules or ETSI TS 119 312, with which was
//!   used said: [`Report::security`] is the assessment against TS 119 312 V2.1.1's tables, each
//!   issue naming the table or clause it failed ([`ts_119_312`], ADR 2896). No national rules are
//!   held, so the report never claims them.
//! - **REQ-4.2-03 e)**: a suite this tree cannot verify does not invalidate the signature; the
//!   result is indeterminate and a warning names the suite and says it is not supported
//!   ([`Cryptography::NotSupported`]).
//!
//! Items e), g) and i) are the report's other sentences already: the signed data is what
//! `/ByteRange` covers and §12.8.1's first question reports, g)'s overall status is the verdict
//! and [`super::Level`], and i) is optional.
//!
//! # Revocation times are the material's statements
//!
//! d) ii) 4) and 5) ask for times, and the verdict's revocation answer ([`super::Valid::revocation`])
//! is what weighed them. The times here are what the document's material states about the signer's
//! certificate — a list by the certificate's issuer, a response by its serial number (RFC 5280
//! section 5.1.2.3 and RFC 6960 section 4.1.1's identifiers) — and they are reported as statements,
//! never as established instants ([`Assurance::StatedBy`]).

use super::{BestSignatureTime, Proof};
use crate::cms::{self, Digest, SignedData};
use crate::policy_rules::signer_attributes;
use crate::revocation::{CertStatus, Evidence, Material, RevocationReason};
use crate::saml::{self, Saml, SignatureCheck};
use crate::signature::{Authenticity, Family, Signature};
use crate::timestamp::{self, Accuracy, AskedAt, Chain, Time};
use crate::trust::TrustAnchors;
use crate::trust::names::{self, Name, NamedAttribute, Unreadable};
use crate::trusted_list::TrustedList;
use crate::trusted_list::time_stamp::{self, ListedAuthority};
use crate::x509::{self, Instant};

/// X.520's `id-at-pseudonym`, `2.5.4.65` — the attribute ETSI EN 319 412-2 clause 4.2.4 has a
/// natural person's certificate state a pseudonym in.
const PSEUDONYM: &str = "2.5.4.65";

/// RFC 8551 section 2.5.2's `smimeCapabilities`, `1.2.840.113549.1.9.15`.
const SMIME_CAPABILITIES: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x0F];

/// How many names of a `subjectAltName` the report reads before it stops and says so.
pub const MAX_ALTERNATIVE_NAMES: usize = 32;

/// REQ-4.5-01 a)'s scope: the check run is the applicability rules checking for European qualified
/// electronic signatures and seals using trusted lists.
///
/// One variant, because this tree runs one such check; a type, so that the clause's text is in
/// one place and a host that words the rest of the report in another language still prints it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Scope {
    /// ETSI TS 119 172-4's rules for EU qualified electronic signatures and seals, checked against
    /// EU Member States' trusted lists in the sense of the European legislation applicable at the
    /// time of signing — Directive 1999/93/EC or Regulation (EU) No 910/2014.
    EuQualifiedUsingTrustedLists,
}

impl Scope {
    /// REQ-4.5-01 a)'s heading, as ETSI TS 119 172-4 V1.1.1 prints it (`doc/questions/A855`).
    #[must_use]
    pub const fn heading(self) -> &'static str {
        match self {
            Self::EuQualifiedUsingTrustedLists => {
                "Signature applicability rules checking (validation rules) for European \
                 qualified electronic signatures/seals using trusted lists"
            }
        }
    }

    /// REQ-4.5-01 a)'s sentence under the heading, as printed; the semicolon that closes the
    /// item in the clause's list is the list's and is not carried.
    #[must_use]
    pub const fn text(self) -> &'static str {
        match self {
            Self::EuQualifiedUsingTrustedLists => {
                "Validation of digital signature to identify whether it can be considered \
                 technically suitable to implement a European qualified electronic \
                 signature/seal using EUMS trusted lists in the sense of the applicable European \
                 legislation at the time of signing, i.e. either Directive 1999/93/EC or \
                 Regulation (EU) No 910/2014"
            }
        }
    }
}

/// REQ-4.5-01 b) and c): the signer as its certificate represents it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signer {
    /// Every attribute of the `subject` field, in the certificate's order.
    pub subject: Vec<NamedAttribute>,
    /// Every name of the `subjectAltName` extension, in its order; empty where there is none.
    pub alternative_names: Vec<Name>,
    /// The `pseudonym` attribute's value, where the subject states one (c)).
    pub pseudonym: Option<String>,
}

/// Where a claimed signing time was stated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimedIn {
    /// RFC 5652 section 11.3's `signingTime` signed attribute.
    SigningTimeAttribute,
    /// The signature dictionary's `/M`, §12.8.1 Table 255.
    Dictionary,
}

/// Which of REQ-4.5-01 d) ii)'s times a [`Timing`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeKind {
    /// 1): the claimed signing time.
    ClaimedSigningTime(ClaimedIn),
    /// 2): a document timestamp over this signature (§12.8.5).
    DocumentTimestamp,
    /// 3): this signature's own signature timestamp (§12.8.3.3.1).
    SignatureTimestamp,
    /// 4): the revocation, or with `suspended` the suspension (`certificateHold`), of the signer's
    /// certificate.
    Revocation {
        /// Whether the reason stated is RFC 5280 section 5.3.1's `certificateHold`.
        suspended: bool,
    },
    /// 5): a certificate revocation list's issue time (`thisUpdate`) about the signer's
    /// certificate, with its `nextUpdate`.
    ListIssued {
        /// The list's `nextUpdate`, where it states one.
        next_update: Option<Instant>,
    },
    /// 5): an OCSP response's `producedAt` about the signer's certificate, with its `nextUpdate`.
    ResponseProduced {
        /// The single response's `nextUpdate`, where it states one.
        next_update: Option<Instant>,
    },
    /// 6): the best signature time (ETSI EN 319 102-1 clause 5.5.4).
    BestSignatureTime,
}

/// REQ-4.5-01 d) iii): a time's evidential relevance and level of assurance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Assurance {
    /// The signer's own statement, which proves nothing about when the signature existed.
    Claimed,
    /// A time-stamp token's `genTime`.
    Token {
        /// Whether the token's validation established it (ADR 1071's four steps).
        established: bool,
        /// Item 1): the time-stamping policy identifier, dotted.
        policy: String,
        /// Item 2): the token's `accuracy`, where it states one.
        accuracy: Option<Accuracy>,
        /// Item 3): whether it is an EU qualified time stamp.
        eu_qualified: EuQualified,
        /// d) i): the time-stamping services the host's lists publish for the token's authority at
        /// its `genTime`; empty where lists were supplied and none does, which is the absence the
        /// item asks to be expressed.
        listed: Vec<ListedAuthority>,
    },
    /// A statement by the issuer of revocation information.
    StatedBy(Evidence),
    /// The best signature time and what fixed it.
    Best(Proof),
}

/// REQ-4.5-01 d) iii) 3) for one token: ETSI TS 119 615 clause 4.7's answer, or why it was not
/// asked.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EuQualified {
    /// The host supplied no trusted list, so there is no list to ask.
    NoListSupplied,
    /// The token's time was not established, and clause 4.7 determines a token's status only
    /// provided that the token is valid (its clause 4.7.1).
    TokenNotEstablished,
    /// The token names no certificate this reader can read, so clause 4.6 has no `CERT`.
    NoCertificate,
    /// Clause 4.7's outputs, or the territory whose list was not supplied.
    Determined(time_stamp::Answer),
}

/// One of REQ-4.5-01 d) ii)'s times, with its assurance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timing {
    /// Which time it is.
    pub kind: TimeKind,
    /// The instant.
    pub at: Instant,
    /// Its relevance and assurance.
    pub assurance: Assurance,
}

/// REQ-4.5-01 f): one attribute of the signer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureAttribute {
    /// The attribute type, dotted.
    pub identifier: String,
    /// Its name in the document that defines it, where this tree knows the identifier.
    pub name: Option<&'static str>,
    /// Whether it is among the signed attributes.
    pub signed: bool,
    /// For a `commitment-type-indication`, the commitment's identifier, dotted, and its name
    /// where it is one of ETSI EN 319 122-1 clause 5.2.3's.
    pub commitment: Option<(String, Option<&'static str>)>,
    /// For a signed `signer-attributes-v2`, the SAML assertions it carries, in its order (ADR
    /// 2922).
    pub assertions: Vec<CarriedAssertion>,
}

/// Which of ETSI EN 319 122-1's members carries an assertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssertionKind {
    /// Clause 5.2.6.2's `claimed-SAML-assertion`, among the claimed attributes.
    Claimed,
    /// Clause 5.2.6.3's `signed-SAML-assertion`, among the signed assertions.
    Signed,
    /// A signed assertion of another identifier, dotted: a form no text defines.
    OtherForm(String),
}

/// One assertion a `signer-attributes-v2` carries, as ETSI EN 319 102-1 clause 5.2.8.4.2.7 makes
/// it available where no rule constrains it: read, its signature checked, and the certificate that
/// verified the signature named. That certificate is not chained to any anchor here, so who it
/// belongs to is what it says of itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarriedAssertion {
    /// Its member.
    pub kind: AssertionKind,
    /// The assertion and its signature; `None` for [`AssertionKind::OtherForm`], or where the
    /// member carries no assertion.
    pub saml: Option<Saml>,
    /// The subject of the certificate whose key verified the signature, where it did and the
    /// certificate's names read.
    pub signer: Option<Signer>,
}

/// REQ-4.5-01 h) and REQ-4.2-03 d) and e): the cryptographic suite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cryptography {
    /// The signature was computed on, under this suite.
    Suite {
        /// The digest algorithm.
        digest: Digest,
        /// The signature algorithm, with the curve where it has one.
        family: Family,
        /// The key's size in bits.
        key_bits: usize,
    },
    /// REQ-4.2-03 e): a suite this tree does not support — the result is indeterminate, never
    /// invalid, and this names the suite.
    NotSupported {
        /// The algorithm, curve, parameters or digest not supported, as the file named it.
        suite: String,
    },
    /// The check never reached a suite: no certificate, an unreadable value, a range not read.
    NotReached,
}

impl Cryptography {
    /// The suite of §12.8.1's second question's answer.
    #[must_use]
    pub fn of(authenticity: &Authenticity) -> Self {
        match authenticity {
            Authenticity::Verified {
                digest,
                family,
                key_bits,
                ..
            }
            | Authenticity::NotUnderThatKey {
                digest,
                family,
                key_bits,
                ..
            } => Self::Suite {
                digest: *digest,
                family: *family,
                key_bits: *key_bits,
            },
            Authenticity::KeyNotVerifiable { algorithm }
            | Authenticity::AlgorithmNotVerifiable { algorithm }
            | Authenticity::UnknownDigest { algorithm } => Self::NotSupported {
                suite: algorithm.clone(),
            },
            Authenticity::CurveNotVerifiable { curve } => Self::NotSupported {
                suite: curve.clone(),
            },
            Authenticity::PssParametersNotVerifiable { statement } => Self::NotSupported {
                suite: format!("RSASSA-PSS ({statement})"),
            },
            _ => Self::NotReached,
        }
    }
}

/// REQ-4.5-01's report on one signature, beside its [`super::Level`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// a).
    pub scope: Scope,
    /// b) and c); the reason where the certificate's names do not read.
    pub signer: Result<Signer, Unreadable>,
    /// d) ii) and iii), in the clause's order.
    pub times: Vec<Timing>,
    /// f), in the file's order.
    pub attributes: Vec<SignatureAttribute>,
    /// h) and REQ-4.2-03 d) and e).
    pub cryptography: Cryptography,
    /// h)'s potential security issues of [`Self::cryptography`]'s suite, against ETSI TS 119 312
    /// at the best signature time and at [`Asked::now`]; `None` where no suite was reached.
    pub security: Option<ts_119_312::Assessment>,
}

/// What [`Report::of`] reads.
#[derive(Debug, Clone, Copy)]
pub struct Asked<'a, 'm> {
    /// The signature.
    pub signature: &'a Signature,
    /// §12.8.1's second question's answer about it.
    pub authenticity: &'a Authenticity,
    /// Its best signature time.
    pub when: BestSignatureTime,
    /// §12.8.5's chain over the document.
    pub chain: &'a Chain,
    /// The signature's position among [`crate::signature::signatures`]'s answers, which is how
    /// the chain names it.
    pub index: usize,
    /// §12.8.4's material.
    pub material: &'a Material<'m>,
    /// The anchors a token's authority is established against.
    pub anchors: &'a TrustAnchors<'a>,
    /// The instant the host supplied, at which a token's authority is asked about — the instant
    /// the best signature time starts from.
    pub now: Instant,
    /// The document's signatures in [`crate::signature::signatures`]'s order, the order
    /// [`Chain`]'s links name them in: where a document timestamp's token is read from.
    pub signatures: &'a [Signature],
    /// The trusted lists the host supplied that read; empty where none was given.
    pub lists: &'a [TrustedList],
}

impl Report {
    /// The report on `asked.signature`.
    #[must_use]
    pub fn of(asked: &Asked<'_, '_>) -> Self {
        let certificate = asked.signature.signer_certificate();
        let parsed = certificate.as_deref().map(x509::parse);
        let signer = match &parsed {
            Some(Ok(certificate)) => signer(certificate),
            _ => Err(Unreadable::Malformed),
        };
        let cms = asked.signature.signed_data().ok();
        let mut times = Vec::new();
        // The imprint digest of each time-stamp token taken as a time (TS 119 312 clause 9.2).
        let mut imprints = Vec::new();
        claimed(asked.signature, cms.as_ref(), &mut times);
        document_timestamps(asked, &mut times, &mut imprints);
        if let Some(cms) = &cms {
            signature_timestamp(cms, asked, &mut times, &mut imprints);
        }
        if let Some(Ok(certificate)) = &parsed {
            revocation_times(certificate, asked.material, &mut times);
        }
        times.push(Timing {
            kind: TimeKind::BestSignatureTime,
            at: asked.when.at(),
            assurance: Assurance::Best(asked.when.proof()),
        });
        let cryptography = Cryptography::of(asked.authenticity);
        let security = match cryptography {
            Cryptography::Suite {
                digest,
                family,
                key_bits,
            } => Some(ts_119_312::Assessment::of(&ts_119_312::Asked {
                digest,
                family,
                key_bits,
                subgroup_bits: match &parsed {
                    Some(Ok(certificate)) => match certificate.public_key {
                        x509::PublicKey::Dsa(key) => Some(key.subgroup_bits()),
                        _ => None,
                    },
                    _ => None,
                },
                signed: asked.when.at(),
                now: asked.now,
            })),
            _ => None,
        }
        .map(|mut assessment| {
            assessment.imprints(&imprints);
            assessment
        });
        Self {
            scope: Scope::EuQualifiedUsingTrustedLists,
            signer,
            times,
            attributes: cms.as_ref().map(attributes).unwrap_or_default(),
            cryptography,
            security,
        }
    }
}

/// b) and c) of one certificate.
///
/// # Errors
///
/// [`Unreadable`] where the subject or the `subjectAltName` does not read, or past
/// [`MAX_ALTERNATIVE_NAMES`].
pub fn signer(certificate: &x509::Certificate<'_>) -> Result<Signer, Unreadable> {
    let subject = names::attributes_of(certificate.subject)?;
    let pseudonym = subject
        .iter()
        .find(|attribute| attribute.kind == PSEUDONYM)
        .map(|attribute| attribute.text.clone().unwrap_or_default());
    let mut alternative_names = Vec::new();
    if let Some(encoded) = certificate.extensions.subject_alt_name {
        let mut reader = crate::der::Reader::new(encoded).map_err(|_| Unreadable::Malformed)?;
        let list = reader
            .next_value()
            .ok()
            .flatten()
            .ok_or(Unreadable::Malformed)?;
        let mut entries = list.children().map_err(|_| Unreadable::Malformed)?;
        while let Some(entry) = entries.next_value().map_err(|_| Unreadable::Malformed)? {
            if alternative_names.len() == MAX_ALTERNATIVE_NAMES {
                return Err(Unreadable::PastBound);
            }
            alternative_names.push(names::general_name(&entry)?);
        }
    }
    Ok(Signer {
        subject,
        alternative_names,
        pseudonym,
    })
}

/// d) ii) 1): the `signingTime` attribute's instant and `/M`'s, each a claim.
fn claimed(signature: &Signature, cms: Option<&SignedData<'_>>, times: &mut Vec<Timing>) {
    let attribute = cms
        .and_then(|cms| cms.attribute(cms::ID_SIGNING_TIME))
        .filter(|attribute| attribute.signed)
        .and_then(|attribute| attribute.first.as_ref())
        .and_then(x509::read_time);
    if let Some(at) = attribute {
        times.push(Timing {
            kind: TimeKind::ClaimedSigningTime(ClaimedIn::SigningTimeAttribute),
            at,
            assurance: Assurance::Claimed,
        });
    }
    if let Some(date) = signature.signed_at_date() {
        times.push(Timing {
            kind: TimeKind::ClaimedSigningTime(ClaimedIn::Dictionary),
            // `Date::instant` counts minutes since the epoch; an `Instant` counts seconds.
            at: Instant::from_unix_seconds(date.instant().saturating_mul(60)),
            assurance: Assurance::Claimed,
        });
    }
}

/// d) ii) 2): every document timestamp whose range covers this signature's revision.
fn document_timestamps(asked: &Asked<'_, '_>, times: &mut Vec<Timing>, imprints: &mut Vec<Digest>) {
    for link in asked
        .chain
        .links
        .iter()
        .filter(|link| link.covers.contains(&asked.index))
    {
        if let Ok(claim) = &link.claim {
            let cms = asked
                .signatures
                .get(link.signature)
                .and_then(|signature| signature.signed_data().ok());
            imprints.push(claim.imprint_digest);
            times.push(Timing {
                kind: TimeKind::DocumentTimestamp,
                at: claim.gen_time,
                assurance: token(claim, &link.time, cms.as_ref(), asked.lists),
            });
        }
    }
}

/// d) ii) 3): the signature timestamp attribute, where it reads and is about this signature.
fn signature_timestamp(
    cms: &SignedData<'_>,
    asked: &Asked<'_, '_>,
    times: &mut Vec<Timing>,
    imprints: &mut Vec<Digest>,
) {
    let Some(Ok(attribute)) = timestamp::signature_timestamp(cms) else {
        return;
    };
    if !attribute.covers_the_signature {
        return;
    }
    // The attribute read above, so the token is there to establish.
    let Some(time) = timestamp::signature_timestamp_established(
        cms,
        asked.anchors,
        asked.lists,
        asked.material,
        AskedAt::TheCallersInstant(asked.now),
    ) else {
        return;
    };
    let token_cms = timestamp::signature_timestamp_token(cms).and_then(Result::ok);
    imprints.push(attribute.claim.imprint_digest);
    times.push(Timing {
        kind: TimeKind::SignatureTimestamp,
        at: attribute.claim.gen_time,
        assurance: token(
            &attribute.claim,
            &time,
            token_cms.as_ref().map(|(token, _)| token),
            asked.lists,
        ),
    });
}

/// d) iii) for a token: items 1) and 2) from the token, 3) by ETSI TS 119 615 clause 4.7 at the
/// instant its authority was asked about and at its `genTime`, and d) i) by clause 4.3 at its
/// `genTime`.
fn token(
    claim: &timestamp::Claim,
    time: &Time,
    cms: Option<&SignedData<'_>>,
    lists: &[TrustedList],
) -> Assurance {
    let listed = cms.map_or_else(Vec::new, |cms| {
        time_stamp::listed_authorities(lists, cms, claim.gen_time)
            .into_iter()
            .map(|(listed, _)| listed)
            .collect()
    });
    let eu_qualified = if lists.is_empty() {
        EuQualified::NoListSupplied
    } else if let Time::Established { asked_at, .. } = time {
        cms.and_then(|cms| {
            time_stamp::qualified_time_stamp_of(lists, cms, claim.gen_time, asked_at.instant())
        })
        .map_or(EuQualified::NoCertificate, EuQualified::Determined)
    } else {
        EuQualified::TokenNotEstablished
    };
    Assurance::Token {
        established: matches!(time, Time::Established { .. }),
        policy: claim.policy.clone(),
        accuracy: claim.accuracy,
        eu_qualified,
        listed,
    }
}

/// d) ii) 4) and 5): what the document's material states about the signer's certificate.
fn revocation_times(
    certificate: &x509::Certificate<'_>,
    material: &Material<'_>,
    times: &mut Vec<Timing>,
) {
    let serial = certificate.serial_number;
    for list in material
        .lists
        .iter()
        .filter(|list| list.issuer == certificate.issuer)
    {
        times.push(Timing {
            kind: TimeKind::ListIssued {
                next_update: list.next_update,
            },
            at: list.this_update,
            assurance: Assurance::StatedBy(Evidence::CertificateRevocationList),
        });
        if let Ok(Some(entry)) = list.entry_for(serial) {
            times.push(Timing {
                kind: TimeKind::Revocation {
                    suspended: entry.reason == Some(RevocationReason::CertificateHold),
                },
                at: entry.revoked_at,
                assurance: Assurance::StatedBy(Evidence::CertificateRevocationList),
            });
        }
    }
    for response in &material.responses {
        for single in response
            .responses
            .iter()
            .filter(|single| same_serial(single.serial_number, serial))
        {
            times.push(Timing {
                kind: TimeKind::ResponseProduced {
                    next_update: single.next_update,
                },
                at: response.produced_at,
                assurance: Assurance::StatedBy(Evidence::OcspResponse),
            });
            if let CertStatus::Revoked { at, reason } = single.status {
                times.push(Timing {
                    kind: TimeKind::Revocation {
                        suspended: reason == Some(RevocationReason::CertificateHold),
                    },
                    at,
                    assurance: Assurance::StatedBy(Evidence::OcspResponse),
                });
            }
        }
    }
}

/// Two serial numbers as the same integer, leading zero octets aside — the comparison
/// [`x509::Certificate::is_named_by`] makes.
fn same_serial(left: &[u8], right: &[u8]) -> bool {
    let trimmed = |octets: &[u8]| {
        let start = octets
            .iter()
            .position(|&octet| octet != 0)
            .unwrap_or(octets.len());
        octets[start..].to_vec()
    };
    trimmed(left) == trimmed(right)
}

/// f): every attribute of the first signer, both sets, in the file's order.
#[must_use]
pub fn attributes(cms: &SignedData<'_>) -> Vec<SignatureAttribute> {
    cms.attributes
        .iter()
        .map(|attribute| SignatureAttribute {
            identifier: x509::dotted(attribute.kind).unwrap_or_default(),
            name: attribute_name(attribute.kind),
            signed: attribute.signed,
            commitment: (attribute.kind == cms::ID_AA_ETS_COMMITMENT_TYPE)
                .then(|| commitment(attribute.first.as_ref()?))
                .flatten(),
            assertions: match (&attribute.first, attribute.signed) {
                (Some(value), true) if attribute.kind == cms::ID_AA_ETS_SIGNER_ATTR_V2 => {
                    assertions(value)
                }
                _ => Vec::new(),
            },
        })
        .collect()
}

/// The SAML assertions a `signer-attributes-v2` value carries: each `claimed-SAML-assertion` among
/// its claimed attributes (ETSI EN 319 122-1 clause 5.2.6.2) and each signed assertion (clause
/// 5.2.6.3's form, or another named). A value that does not read carries none here; the departure
/// it is is §12.8.3.4.3 (j)'s report.
fn assertions(value: &crate::der::Value<'_>) -> Vec<CarriedAssertion> {
    let Ok(carried) = signer_attributes::carried_by_v2(value) else {
        return Vec::new();
    };
    let shown = |kind, saml: Saml| {
        let signer = match &saml.signature {
            SignatureCheck::Verified(der) => x509::parse(der)
                .ok()
                .and_then(|certificate| signer(&certificate).ok()),
            _ => None,
        };
        CarriedAssertion {
            kind,
            saml: Some(saml),
            signer,
        }
    };
    let mut out = Vec::new();
    for (kind, values) in carried.claimed.iter().zip(&carried.claimed_values) {
        if *kind != saml::ID_AA_ETS_CLAIMED_SAML {
            continue;
        }
        // Clause 5.2.6.2: exactly one value, a `ClaimedSAMLAssertion ::= OCTET STRING`.
        let octets = values
            .children()
            .ok()
            .and_then(|mut members| {
                let first = members.next_value().ok().flatten()?;
                members.next_value().ok()?.is_none().then_some(first)
            })
            .filter(|value| value.identifier == crate::der::OCTET_STRING)
            .map(|value| value.contents);
        let saml = octets.map_or_else(
            || Saml {
                assertion: Err(saml::Unreadable::Malformed(
                    "ClaimedSAMLAssertion OCTET STRING",
                )),
                signature: SignatureCheck::Unsigned,
            },
            Saml::of,
        );
        out.push(shown(AssertionKind::Claimed, saml));
    }
    for assertion in &carried.assertions {
        if assertion.identifier != saml::ID_ETS_SIGNED_SAML {
            out.push(CarriedAssertion {
                kind: AssertionKind::OtherForm(
                    x509::dotted(assertion.identifier).unwrap_or_default(),
                ),
                saml: None,
                signer: None,
            });
            continue;
        }
        match (assertion.saml_octets(), assertion.assertion) {
            (Some(octets), _) => out.push(shown(AssertionKind::Signed, Saml::of(octets))),
            (None, None) => out.push(CarriedAssertion {
                kind: AssertionKind::Signed,
                saml: None,
                signer: None,
            }),
            (None, Some(_)) => out.push(shown(
                AssertionKind::Signed,
                Saml {
                    assertion: Err(saml::Unreadable::Malformed(
                        "SignedSAMLAssertion OCTET STRING",
                    )),
                    signature: SignatureCheck::Unsigned,
                },
            )),
        }
    }
    out
}

/// `CommitmentTypeIndication ::= SEQUENCE { commitmentTypeId OBJECT IDENTIFIER, ... }` (ETSI EN
/// 319 122-1 clause 5.2.3): the identifier and, for the six that clause defines, its name.
fn commitment(value: &crate::der::Value<'_>) -> Option<(String, Option<&'static str>)> {
    let identifier = value.children().ok()?.next_value().ok()??;
    let oid = identifier.object_identifier()?;
    let dotted = x509::dotted(oid)?;
    let name = match dotted.as_str() {
        "1.2.840.113549.1.9.16.6.1" => Some("proof of origin"),
        "1.2.840.113549.1.9.16.6.2" => Some("proof of receipt"),
        "1.2.840.113549.1.9.16.6.3" => Some("proof of delivery"),
        "1.2.840.113549.1.9.16.6.4" => Some("proof of sender"),
        "1.2.840.113549.1.9.16.6.5" => Some("proof of approval"),
        "1.2.840.113549.1.9.16.6.6" => Some("proof of creation"),
        _ => None,
    };
    Some((dotted, name))
}

/// The name of an attribute type this tree reads, under the document that defines it.
#[must_use]
pub fn attribute_name(kind: &[u8]) -> Option<&'static str> {
    Some(match kind {
        cms::ID_CONTENT_TYPE => "content-type (RFC 5652)",
        cms::ID_MESSAGE_DIGEST => "message-digest (RFC 5652)",
        cms::ID_SIGNING_TIME => "signing-time (RFC 5652)",
        cms::ID_COUNTERSIGNATURE => "countersignature (RFC 5652)",
        cms::ID_AA_SIGNING_CERTIFICATE => "signing-certificate (RFC 5035)",
        cms::ID_AA_SIGNING_CERTIFICATE_V2 => "signing-certificate-v2 (RFC 5035)",
        cms::ID_AA_CONTENT_HINT => "content-hints (RFC 2634)",
        cms::ID_AA_CONTENT_IDENTIFIER => "content-identifier (RFC 2634)",
        cms::ID_AA_CONTENT_REFERENCE => "content-reference (RFC 2634)",
        cms::ID_AA_ETS_SIGNER_LOCATION => "signer-location (ETSI EN 319 122-1)",
        cms::ID_AA_ETS_CONTENT_TIMESTAMP => "content-time-stamp (ETSI EN 319 122-1)",
        cms::ID_AA_ETS_SIGNER_ATTR_V2 => "signer-attributes-v2 (ETSI EN 319 122-1)",
        cms::ID_AA_ETS_SIGNER_ATTR => "signer-attributes (RFC 5126)",
        cms::ID_AA_ETS_SIG_POLICY_ID => "signature-policy-identifier (ETSI EN 319 122-1)",
        cms::ID_AA_ETS_SIG_POLICY_STORE => "signature-policy-store (ETSI EN 319 122-1)",
        cms::ID_AA_ETS_COMMITMENT_TYPE => "commitment-type-indication (ETSI EN 319 122-1)",
        cms::ID_AA_ETS_MIME_TYPE => "mime-type (ETSI EN 319 122-1)",
        cms::ID_AA_TIME_STAMP_TOKEN => "signature-time-stamp (RFC 3161)",
        SMIME_CAPABILITIES => "smime-capabilities (RFC 8551)",
        _ => return None,
    })
}

pub mod ts_119_312;

#[cfg(test)]
mod tests;
