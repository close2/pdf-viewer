//! ETSI TS 119 612's trusted lists, read, and ETSI TS 119 615 clause 4.3's search of one for the
//! services that issued a certificate.
//!
//! ISO 32000-2 §12.8.1 leaves whom a signer is to be believed by to the validation parameters a
//! handler is given, and RFC 5280 section 6.1.1 makes the trust anchors input (d), "a matter of
//! policy". In the European Union the policy is published: each Member State's supervisory body
//! keeps a trusted list of the trust service providers it supervises, in TS 119 612's XML, and the
//! European Commission's list of trusted lists points at them. ETSI TS 119 172-4 clause 4.2
//! (REQ-4.2-03 a)) makes the anchors for validating an EU qualified signature the service digital
//! identities TS 119 615 clause 4.3 returns for the signing certificate, the service type
//! `CA/QC`, and the certificate's own `notBefore`. This module is those two steps; the ETSI texts
//! are cited by clause and paraphrased, never quoted (ADR 1085).
//!
//! # What this program is handed, and what it does not do with it
//!
//! A trusted list reaches this crate as bytes a host read — a file a person or an organisation
//! keeps, or one the host fetched under its own transmission rule (ADR 1728, `doc/questions/A130`).
//! This crate opens no socket and reads no file (`CLAUDE.md` principle 3), so the list of trusted
//! lists is never walked here.
//!
//! **A list's own XML signature is verified where the host supplied what to verify it against.**
//! TS 119 615 clause 4.3's first step (PRO-4.3.4-01) authenticates the list by clause 4.2, against
//! the certificates the list of trusted lists names for it, which clause 4.1 authenticates in turn
//! against the Official Journal's certificates; [`authentication`] runs both over the files a host
//! supplied, and [`Lists::read`] refuses by name a list they do not authenticate. A host that
//! supplied neither the list of trusted lists nor the Official Journal's certificates gave nothing
//! to authenticate against, so its lists are believed on its word — the footing RFC 5280 section
//! 6.1 gives any anchor, "delivered to the path processing procedure by some trustworthy
//! out-of-band procedure" — and every answer built on one says so
//! ([`Authentication::VouchedForByTheHost`]). ADRs 2834 and 2842.
//!
//! # The bounds
//!
//! A list is a stranger's XML. [`MAX_LIST_OCTETS`] bounds what is read at all and
//! [`MAX_ENTRIES`] the elements and namespace declarations; a document type declaration is refused,
//! which is where entity expansion would live (`crate::xml`).

use crate::signature::Signature;
use crate::trust::{Trust, TrustAnchors};
use crate::verdict::Listing;
use crate::x509::{self, Certificate, Instant};
use crate::xml::{self, Element, Tree, XmlError, base64, date_time};

pub mod authentication;

pub use authentication::Outcome;

/// TS 119 612 Annex C's target namespace for the list's own elements.
pub const NAMESPACE: &str = "http://uri.etsi.org/02231/v2#";

/// TS 119 612 clause 5.5.1.1's identifier for a CA issuing EU qualified certificates — the
/// `TLS-Sti` ETSI TS 119 172-4 REQ-4.2-03 a) ii) searches with.
pub const CA_QC: &str = "http://uri.etsi.org/TrstSvc/Svctype/CA/QC";

/// TS 119 612 clause 5.5.4's status of a qualified service that is granted.
pub const GRANTED: &str = "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/granted";

/// TS 119 612 clause 5.5.9.4's `additionalServiceInformation` values TS 119 615 PRO-4.3.4-05 to
/// -10 look for, in the order of those requirements' three pairs.
pub const FOR_TYPES: [&str; 3] = [
    "http://uri.etsi.org/TrstSvc/TrustedList/SvcInfoExt/ForeSignatures",
    "http://uri.etsi.org/TrstSvc/TrustedList/SvcInfoExt/ForeSeals",
    "http://uri.etsi.org/TrstSvc/TrustedList/SvcInfoExt/ForWebSiteAuthentication",
];

/// How many octets of a list are read at all.
///
/// TS 119 612 states no ceiling. A Member State's list is a few megabytes at the most; thirty-two
/// is a bound on work over a stranger's input, not a reading of the standard.
pub const MAX_LIST_OCTETS: usize = 32 << 20;

/// How many elements and namespace declarations a list may hold.
pub const MAX_ENTRIES: usize = 1 << 20;

/// How many services a list may state, history instances included, before it is refused.
pub const MAX_SERVICES: usize = 1 << 16;

/// Territories TS 119 615 PRO-4.3.4-01 accepts: the Member States as TS 119 612 clause 5.1.5
/// spells them (`EL` for Greece), and `GB` and `UK`.
const EU_TERRITORIES: [&str; 29] = [
    "AT", "BE", "BG", "CY", "CZ", "DE", "DK", "EE", "EL", "ES", "FI", "FR", "HR", "HU", "IE", "IT",
    "LT", "LU", "LV", "MT", "NL", "PL", "PT", "RO", "SE", "SI", "SK", "GB", "UK",
];

/// Why a list was not read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum TrustedListError {
    /// Longer than [`MAX_LIST_OCTETS`].
    #[error(
        "the trusted list is {octets} octets long, past the {MAX_LIST_OCTETS} this program reads"
    )]
    TooLarge {
        /// Its length.
        octets: usize,
    },
    /// The XML cannot be read: not UTF-8, not well-formed, or declaring a document type.
    #[error("the trusted list's XML cannot be read: {0}")]
    Xml(String),
    /// Elements nest deeper than this crate's DER depth bound.
    #[error("the trusted list nests its elements deeper than this program reads")]
    NestedTooDeep,
    /// More than [`MAX_ENTRIES`] elements, or [`MAX_SERVICES`] services.
    #[error("the trusted list states more entries than this program reads")]
    TooManyEntries,
    /// The root is not TS 119 612 clause 5.2's `TrustServiceStatusList`.
    #[error("the document is not an ETSI TS 119 612 trusted list")]
    NotATrustedList,
    /// A field TS 119 612 clause 5 says shall be present is missing or not of its format; the
    /// element is named.
    #[error("the trusted list's {0} is missing or not what ETSI TS 119 612 clause 5 gives it")]
    Malformed(&'static str),
    /// ETSI TS 119 615 clause 4.2 did not authenticate it, or clause 4.1 did not authenticate the
    /// list of trusted lists it would have been authenticated under; the indications are said.
    #[error("ETSI TS 119 615 did not authenticate the list: {0}")]
    NotAuthenticated(Outcome),
}

impl From<XmlError> for TrustedListError {
    fn from(error: XmlError) -> Self {
        match error {
            XmlError::Unreadable(reason) => Self::Xml(reason),
            XmlError::NestedTooDeep => Self::NestedTooDeep,
            XmlError::TooManyEntries => Self::TooManyEntries,
        }
    }
}

/// One status a service held: its current one (TS 119 612 clause 5.5) or a previous one (a
/// service history instance, clause 5.6). The two carry the same fields, which is why one type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceState {
    /// Clause 5.5.1's (or 5.6.1's) service type identifier.
    pub type_identifier: String,
    /// Clause 5.5.2's service name, the first language the list gives.
    pub name: String,
    /// Clause 5.5.3's `X509Certificate` representations of the service digital identity, decoded.
    /// Clause 5.6.3 excludes certificates from a history instance, so there this is empty.
    pub certificates: Vec<Vec<u8>>,
    /// Clause 5.5.4's current status (or 5.6.4's previous status), a URI.
    pub status: String,
    /// Clause 5.5.5's (or 5.6.5's) instant that status became effective.
    pub since: Instant,
    /// Clause 5.5.9.4's `additionalServiceInformation` URIs.
    pub additional_information: Vec<String>,
}

/// One listed service: its current status and, newest first as clause 5.5.10 orders them, the
/// previous ones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Service {
    /// Clause 5.5's service information.
    pub current: ServiceState,
    /// Clause 5.5.10's service history instances, in the list's order.
    pub history: Vec<ServiceState>,
}

/// TS 119 612 clause 5.4's trust service provider, as far as clause 4.3's outputs name it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provider {
    /// Clause 5.4.1's TSP name, the first language the list gives.
    pub name: String,
    /// Clause 5.4.2's TSP trade name, where the list states one.
    pub trade_name: Option<String>,
    /// Clause 5.4.6's services.
    pub services: Vec<Service>,
}

/// A TS 119 612 trusted list, as far as TS 119 615 clause 4.3 reads one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedList {
    /// Clause 5.3.10's scheme territory.
    pub territory: String,
    /// Clause 5.3.2's sequence number.
    pub sequence_number: u64,
    /// Clause 5.3.14's list issue date and time.
    pub issued: Instant,
    /// Clause 5.3.15's next update; `None` for a closed list, which the clause spells as an empty
    /// value.
    pub next_update: Option<Instant>,
    /// Clause 5.3.18's providers.
    pub providers: Vec<Provider>,
    /// Clause 5.3.7's scheme information URIs, in the list's order, every language's.
    pub scheme_information_uris: Vec<String>,
    /// Clause 5.3.13's pointers to other lists.
    pub pointers: Vec<Pointer>,
    /// How far this list's authenticity was established: what [`TrustedList::read`] alone can
    /// say, [`Authentication::VouchedForByTheHost`], until [`Lists::read`] runs TS 119 615
    /// clauses 4.1 and 4.2.
    pub authentication: Authentication,
}

/// One tuple of TS 119 612 clause 5.3.13's pointers to other lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pointer {
    /// Item a): the URI of the pointed-to list's machine-processable form (`TSLLocation`).
    pub location: String,
    /// Item b): the digital identities of the pointed-to list's issuer, the clause 5.5.3
    /// `X509Certificate`s decoded.
    pub certificates: Vec<Vec<u8>>,
    /// Item c)'s scheme territory qualifier, where stated.
    pub territory: Option<String>,
    /// Item c)'s MIME type qualifier (clause 6.2), where stated.
    pub mime_type: Option<String>,
}

/// Clause 6.2's media type for a list's XML form.
pub const XML_MIME_TYPE: &str = "application/vnd.etsi.tsl+xml";

/// The namespace of the additional types clause 5.3.13's MIME type qualifier is written in
/// (Annex C's `19612_additionaltypes_xsd.xsd`).
pub const ADDITIONAL_TYPES: &str = "http://uri.etsi.org/02231/v2/additionaltypes#";

/// How far a list's authenticity was established before anything was built on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Authentication {
    /// TS 119 615 clauses 4.1 and 4.2 were not run, because the host supplied neither the list of
    /// trusted lists nor the Official Journal's certificates: the list is believed because the host
    /// that supplied it vouched for it (module documentation, ADR 2842).
    VouchedForByTheHost,
    /// Clause 4.2 passed: the list's signature verified under a certificate the authenticated list
    /// of trusted lists names for its territory, and that list passed clause 4.1 against the
    /// Official Journal's certificates.
    Authenticated {
        /// The sequence number (TS 119 612 clause 5.3.2) of the list of trusted lists that
        /// authenticated it.
        lotl_sequence_number: u64,
    },
}

impl TrustedList {
    /// Reads a list's octets.
    ///
    /// # Errors
    ///
    /// [`TrustedListError`]: the bounds, XML that cannot be read, a root that is not a list, and a
    /// field clause 5 requires that is missing or not of its format.
    pub fn read(octets: &[u8]) -> Result<Self, TrustedListError> {
        if octets.len() > MAX_LIST_OCTETS {
            return Err(TrustedListError::TooLarge {
                octets: octets.len(),
            });
        }
        let mut remaining = MAX_ENTRIES;
        let tree = xml::read_tree(octets, &mut remaining)?;
        let reader = Reader { tree: &tree };
        let root = reader.element(0)?;
        if !is(root, "TrustServiceStatusList") {
            return Err(TrustedListError::NotATrustedList);
        }
        reader.list(root)
    }

    /// Whether clause 5.3.10's territory is one TS 119 615 PRO-4.3.4-01 accepts.
    #[must_use]
    pub fn is_eu_member_state(&self) -> bool {
        EU_TERRITORIES.contains(&self.territory.as_str())
    }
}

impl Service {
    /// TS 119 615 PRO-4.3.4-03 b)'s `SI-at-Date-time`: the current information where `at` is at
    /// or after its starting instant, otherwise the first history instance whose starting instant
    /// is at or before `at`; `None` where the service had no status yet at `at`.
    #[must_use]
    pub fn at(&self, at: Instant) -> Option<&ServiceState> {
        if at.unix_seconds() >= self.current.since.unix_seconds() {
            return Some(&self.current);
        }
        self.history
            .iter()
            .find(|state| state.since.unix_seconds() <= at.unix_seconds())
    }

    /// PRO-4.3.4-03A: the history instances descend by starting instant (TS 119 612 clause
    /// 5.5.10's order) below the current one, and no two share one.
    fn history_is_ordered(&self) -> Result<(), Fault> {
        let mut later = self.current.since.unix_seconds();
        for state in &self.history {
            let since = state.since.unix_seconds();
            if since == later {
                return Err(Fault::HistorySharesAnInstant);
            }
            if since > later {
                return Err(Fault::HistoryOutOfOrder);
            }
            later = since;
        }
        Ok(())
    }
}

/// TS 119 615 clause 4.3.3's `SI-Status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// `PROCESS_PASSED`.
    Passed,
    /// `PROCESS_FAILED`.
    Failed,
}

/// The `SI-Sub-Status` indications clause 4.3.4 defines, and the two errors PRO-4.3.4-03A
/// reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Fault {
    /// PRO-4.3.4-01 b): `Provided_country_code_not_representing_an_EU_MS`.
    NotAnEuMemberState,
    /// PRO-4.3.4-03A a): a matched service's history instances are not in descending order.
    HistoryOutOfOrder,
    /// PRO-4.3.4-03A b): two of a matched service's statuses start at the same instant.
    HistorySharesAnInstant,
    /// PRO-4.3.4-05, -07 and -09: `WARNING_T1_DUPLICATION`, `T2` and `T3` — two matched services
    /// for the same type of certificate ([`FOR_TYPES`]' index) with identical statuses.
    DuplicationWarning(usize),
    /// PRO-4.3.4-06, -08 and -10: `ERROR_T1_DUPLICATION` and its siblings — the same with
    /// statuses that differ.
    DuplicationError(usize),
    /// PRO-4.3.4-11: `ERROR_TSP_CONFLICT`, matched services of different providers.
    ProviderConflict,
}

/// One tuple of clause 4.3.3's `SI-Results`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Matched<'l> {
    /// `TSP-Name` and `TSP-Trade-Name`, with the provider's other services.
    pub provider: &'l Provider,
    /// `SI-full`.
    pub service: &'l Service,
    /// `SI-at-Date-time`; `None` where the service had no status yet at the instant asked about.
    pub at_date_time: Option<&'l ServiceState>,
}

/// TS 119 615 clause 4.3's three outputs for one list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Matching<'l> {
    /// `SI-Status`.
    pub status: Status,
    /// `SI-Sub-Status`.
    pub faults: Vec<Fault>,
    /// `SI-Results`.
    pub results: Vec<Matched<'l>>,
}

/// TS 119 615 clause 4.3: the services of `list` whose type is `service_type` and whose digital
/// identity issued `certificate`, as of `at`.
///
/// PRO-4.3.4-03 ii)'s test is RFC 5280 section 6.1 with the identity as the only anchor, at `at`,
/// under the section's default inputs, over `pool` (the certificates the signature carries) — or
/// the identity's key and subject being the certificate's own. Section 6.1 is run without
/// revocation material, which clause 4.3 does not name; whether the certificate was revoked is the
/// validation's, after this search has chosen its anchors.
///
/// PRO-4.3.4-01's authentication of the list by clause 4.2 is [`Lists::read`]'s, before a list
/// reaches this search; the territory test it opens with is here.
#[must_use]
pub fn matching<'l>(
    list: &'l TrustedList,
    certificate: &Certificate<'_>,
    pool: &[Certificate<'_>],
    service_type: &str,
    at: Instant,
) -> Matching<'l> {
    if !list.is_eu_member_state() {
        return Matching {
            status: Status::Failed,
            faults: vec![Fault::NotAnEuMemberState],
            results: Vec::new(),
        };
    }
    let mut results = Vec::new();
    for provider in &list.providers {
        for service in &provider.services {
            if service.current.type_identifier == service_type
                && identity_issued(&service.current, certificate, pool, at)
            {
                results.push(Matched {
                    provider,
                    service,
                    at_date_time: service.at(at),
                });
            }
        }
    }
    let mut faults = Vec::new();
    for matched in &results {
        if let Err(fault) = matched.service.history_is_ordered() {
            return Matching {
                status: Status::Failed,
                faults: vec![fault],
                results,
            };
        }
    }
    let mut status = Status::Passed;
    for (index, for_type) in FOR_TYPES.iter().enumerate() {
        let statuses: Vec<&str> = results
            .iter()
            .filter_map(|matched| matched.at_date_time)
            .filter(|state| {
                state
                    .additional_information
                    .iter()
                    .any(|uri| uri == for_type)
            })
            .map(|state| state.status.as_str())
            .collect();
        if statuses.len() >= 2 {
            faults.push(
                if statuses.windows(2).all(|pair| pair.first() == pair.last()) {
                    Fault::DuplicationWarning(index)
                } else {
                    Fault::DuplicationError(index)
                },
            );
        }
    }
    if results
        .windows(2)
        .any(|pair| pair.first().map(|m| &m.provider.name) != pair.last().map(|m| &m.provider.name))
    {
        faults.push(Fault::ProviderConflict);
        status = Status::Failed;
    }
    Matching {
        status,
        faults,
        results,
    }
}

/// PRO-4.3.4-03 ii) for one service's digital identity.
fn identity_issued(
    state: &ServiceState,
    certificate: &Certificate<'_>,
    pool: &[Certificate<'_>],
    at: Instant,
) -> bool {
    let identities: Vec<Certificate<'_>> = state
        .certificates
        .iter()
        .filter_map(|der| x509::parse(der).ok())
        .collect();
    if identities.iter().any(|identity| {
        identity.subject == certificate.subject
            && identity.public_key_bits == certificate.public_key_bits
    }) {
        return true;
    }
    if identities.is_empty() {
        return false;
    }
    let anchors = TrustAnchors::of(&identities);
    matches!(
        crate::trust::validate(
            certificate,
            pool,
            &anchors,
            &crate::revocation::Material::none(),
            at
        ),
        Trust::Anchored { .. }
    )
}

/// ETSI TS 119 172-4 REQ-4.2-03 a) for one signature: per list, the services clause 4.3 returns
/// for the signing certificate with `TLS-Sti` [`CA_QC`] at the certificate's `notBefore`.
///
/// Empty where the signature names no certificate this reader can read, or the certificate states
/// no validity.
#[must_use]
pub fn qualified_issuers<'l>(
    lists: &'l [TrustedList],
    signature: &Signature,
) -> Vec<(&'l TrustedList, Matching<'l>)> {
    let Some(der) = signature.signer_certificate() else {
        return Vec::new();
    };
    let Ok(certificate) = x509::parse(&der) else {
        return Vec::new();
    };
    let Some(validity) = certificate.validity else {
        return Vec::new();
    };
    let carried = signature.carried_certificates();
    let pool: Vec<Certificate<'_>> = carried
        .iter()
        .filter_map(|der| x509::parse(der).ok())
        .filter(|candidate| candidate.tbs != certificate.tbs)
        .collect();
    lists
        .iter()
        .map(|list| {
            (
                list,
                matching(list, &certificate, &pool, CA_QC, validity.not_before),
            )
        })
        .collect()
}

/// ETSI TS 119 172-4 REQ-4.2-03 a)'s anchors for one signature: per service clause 4.3 returned
/// with `SI-Status` `PROCESS_PASSED`, the service's digital identity as DER and the
/// [`Listing`] a verdict reached through it carries.
///
/// One entry per service rather than one set for all of them, so that the verdict can name the
/// service whose identity the path ended at.
#[must_use]
pub fn listed_anchors(
    lists: &[TrustedList],
    signature: &Signature,
) -> Vec<(Listing, Vec<Vec<u8>>)> {
    let mut anchors = Vec::new();
    // EN 319 412-5's statements are the signing certificate's, the same for every service that
    // issued it, so they are read once.
    let statements =
        signature
            .signer_certificate()
            .map_or(crate::qc_statement::Stated::Nothing, |der| {
                x509::parse(&der).map_or(crate::qc_statement::Stated::Nothing, |certificate| {
                    crate::qc_statement::Stated::of(&certificate)
                })
            });
    for (list, found) in qualified_issuers(lists, signature) {
        if found.status != Status::Passed {
            continue;
        }
        for matched in found.results {
            anchors.push((
                Listing {
                    territory: list.territory.clone(),
                    sequence_number: list.sequence_number,
                    provider: matched.provider.name.clone(),
                    service: matched.service.current.name.clone(),
                    service_type: matched.service.current.type_identifier.clone(),
                    status_at_issue: matched.at_date_time.map(|state| state.status.clone()),
                    authentication: list.authentication,
                    statements: statements.clone(),
                },
                matched.service.current.certificates.clone(),
            ));
        }
    }
    anchors
}

/// The trusted lists a host holds: each list's octets with the name the host knows it by, and the
/// sentence saying where they came from.
///
/// [`crate::trust::Supply`]'s shape for [`crate::trust::Supply`]'s reason: the octets are owned
/// here, a verdict is not separable from *whose* lists they were, and the default is none — a host
/// that names no list changes nothing (ADR 2834). Off by default under `CLAUDE.md` principle 3: a
/// host turns it on, never a document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lists {
    lists: Vec<(String, Vec<u8>)>,
    source: String,
}

impl Lists {
    /// No list — the default.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// The lists a host read, each with its name, and where they came from.
    #[must_use]
    pub fn of(lists: Vec<(String, Vec<u8>)>, source: String) -> Self {
        Self { lists, source }
    }

    /// Whether no list was supplied.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lists.is_empty()
    }

    /// Where the host says the lists came from.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The octets, with the name the host knows each list by.
    #[must_use]
    pub fn lists(&self) -> &[(String, Vec<u8>)] {
        &self.lists
    }

    /// The lists that read and were authenticated, and the ones that were not with why — named
    /// rather than dropped (trap 5), because a verdict computed under fewer lists than a host named
    /// is not the one it asked for.
    ///
    /// TS 119 615 clauses 4.1 and 4.2 run where the host supplied the list of trusted lists or a
    /// parameter of GPR-4.0-02's ([`authentication`]), at `at`; a list they do not authenticate is
    /// refused with [`TrustedListError::NotAuthenticated`], and the lists of territory `EU` (the
    /// list of trusted lists and its pivots) are clause 4.1's inputs and not searched. Where the
    /// host supplied neither, every list that reads is returned as
    /// [`Authentication::VouchedForByTheHost`].
    #[must_use]
    pub fn read(&self, at: Instant) -> Reading {
        let configuration = authentication::Configuration::of(self);
        let lotl_name = authentication::file_name(&configuration.ojeu_lotl_loc);
        let run = configuration.supplied || self.lists.iter().any(|(name, _)| name == lotl_name);
        let lotl = run.then(|| authentication::lotl(self, &configuration, at));
        let mut reading = Reading::default();
        for (name, octets) in &self.lists {
            if run && authentication::Configuration::names(name) {
                continue;
            }
            let mut list = match TrustedList::read(octets) {
                Ok(list) => list,
                Err(error) => {
                    reading.refused.push((name.clone(), error));
                    continue;
                }
            };
            match &lotl {
                None => reading.lists.push(list),
                Some(_) if list.territory == "EU" => {}
                // PRO-4.2.4-02: the LOTL's indications become the list's.
                Some(Err(outcome)) => {
                    let mut sub_status = vec![outcome.status.to_owned()];
                    sub_status.extend(outcome.sub_status.iter().cloned());
                    reading.refused.push((
                        name.clone(),
                        TrustedListError::NotAuthenticated(Outcome {
                            status: "TL_VERIFICATION_FAILED",
                            sub_status,
                        }),
                    ));
                }
                Some(Ok(authenticated)) => {
                    match authentication::member_state(&mut list, octets, authenticated, at) {
                        Ok(outcome) => {
                            if !outcome.sub_status.is_empty() {
                                reading.warnings.push((name.clone(), outcome));
                            }
                            reading.lists.push(list);
                        }
                        Err(outcome) => reading
                            .refused
                            .push((name.clone(), TrustedListError::NotAuthenticated(outcome))),
                    }
                }
            }
        }
        reading.lotl = lotl.map(|outcome| match outcome {
            Ok(authenticated) => (authenticated.list.sequence_number, authenticated.outcome),
            Err(outcome) => (0, outcome),
        });
        reading
    }
}

/// What [`Lists::read`] made of the host's files.
#[derive(Debug, Default)]
pub struct Reading {
    /// The lists that read and, where clauses 4.1 and 4.2 ran, were authenticated.
    pub lists: Vec<TrustedList>,
    /// The files that are not lists this reader uses, by the host's name, with why.
    pub refused: Vec<(String, TrustedListError)>,
    /// Clause 4.1's outcome where it ran, with the sequence number of the list of trusted lists it
    /// authenticated (zero where it authenticated none).
    pub lotl: Option<(u64, Outcome)>,
    /// The authenticated lists whose clause 4.2 outcome carries a warning, by the host's name.
    pub warnings: Vec<(String, Outcome)>,
}

/// Whether `element` is the list namespace's `local`.
fn is(element: &Element, local: &str) -> bool {
    element.namespace == NAMESPACE && element.local == local
}

/// The second step: the tree's elements read as clause 5's fields.
struct Reader<'t> {
    tree: &'t Tree,
}

impl<'t> Reader<'t> {
    fn element(&self, index: usize) -> Result<&'t Element, TrustedListError> {
        self.tree
            .elements
            .get(index)
            .ok_or(TrustedListError::Malformed("element"))
    }

    /// Every child of `parent` in the list's namespace named `local`.
    fn all(&self, parent: &Element, local: &str) -> Vec<&'t Element> {
        parent
            .children
            .iter()
            .filter_map(|&index| self.tree.elements.get(index))
            .filter(|child| is(child, local))
            .collect()
    }

    fn first(&self, parent: &Element, local: &str) -> Option<&'t Element> {
        self.all(parent, local).into_iter().next()
    }

    fn required(
        &self,
        parent: &Element,
        local: &'static str,
    ) -> Result<&'t Element, TrustedListError> {
        self.first(parent, local)
            .ok_or(TrustedListError::Malformed(local))
    }

    fn text(&self, parent: &Element, local: &'static str) -> Result<String, TrustedListError> {
        let text = self.required(parent, local)?.text.trim();
        if text.is_empty() {
            return Err(TrustedListError::Malformed(local));
        }
        Ok(text.to_owned())
    }

    fn instant(&self, parent: &Element, local: &'static str) -> Result<Instant, TrustedListError> {
        date_time(&self.text(parent, local)?).ok_or(TrustedListError::Malformed(local))
    }

    /// Clause 5.1.4's multilingual names: the first `Name` child of the element.
    fn name(&self, parent: &Element, local: &'static str) -> Result<String, TrustedListError> {
        let names = self.required(parent, local)?;
        self.text(names, "Name")
            .map_err(|_| TrustedListError::Malformed(local))
    }

    fn list(&self, root: &Element) -> Result<TrustedList, TrustedListError> {
        let scheme = self.required(root, "SchemeInformation")?;
        let sequence_number = self
            .text(scheme, "TSLSequenceNumber")?
            .parse::<u64>()
            .map_err(|_| TrustedListError::Malformed("TSLSequenceNumber"))?;
        let next = self.required(scheme, "NextUpdate")?;
        let next_update = match self.first(next, "dateTime") {
            Some(value) if !value.text.trim().is_empty() => Some(
                date_time(value.text.trim()).ok_or(TrustedListError::Malformed("NextUpdate"))?,
            ),
            _ => None,
        };
        let mut services = 0_usize;
        let mut providers = Vec::new();
        // Clause 5.3.18 makes the provider list optional: a list with nobody on it yet.
        if let Some(list) = self.first(root, "TrustServiceProviderList") {
            for provider in self.all(list, "TrustServiceProvider") {
                providers.push(self.provider(provider, &mut services)?);
            }
        }
        let scheme_information_uris = self
            .first(scheme, "SchemeInformationURI")
            .map(|uris| {
                self.all(uris, "URI")
                    .into_iter()
                    .map(|uri| uri.text.trim().to_owned())
                    .collect()
            })
            .unwrap_or_default();
        let mut pointers = Vec::new();
        if let Some(list) = self.first(scheme, "PointersToOtherTSL") {
            for pointer in self.all(list, "OtherTSLPointer") {
                pointers.push(self.pointer(pointer)?);
            }
        }
        Ok(TrustedList {
            territory: self.text(scheme, "SchemeTerritory")?,
            sequence_number,
            issued: self.instant(scheme, "ListIssueDateTime")?,
            next_update,
            providers,
            scheme_information_uris,
            pointers,
            authentication: Authentication::VouchedForByTheHost,
        })
    }

    /// Clause 5.3.13's tuple: the location, the issuer's certificates, and the qualifiers in
    /// `AdditionalInformation`'s `OtherInformation` children.
    fn pointer(&self, pointer: &Element) -> Result<Pointer, TrustedListError> {
        let mut certificates = Vec::new();
        if let Some(identities) = self.first(pointer, "ServiceDigitalIdentities") {
            for identity in self.all(identities, "ServiceDigitalIdentity") {
                for digital in self.all(identity, "DigitalId") {
                    for certificate in self.all(digital, "X509Certificate") {
                        certificates.push(
                            base64(&certificate.text)
                                .ok_or(TrustedListError::Malformed("X509Certificate"))?,
                        );
                    }
                }
            }
        }
        let mut territory = None;
        let mut mime_type = None;
        if let Some(additional) = self.first(pointer, "AdditionalInformation") {
            for other in self.all(additional, "OtherInformation") {
                for &index in &other.children {
                    let qualifier = self.element(index)?;
                    let value = Some(qualifier.text.trim().to_owned());
                    if is(qualifier, "SchemeTerritory") {
                        territory = territory.or(value);
                    } else if qualifier.namespace == ADDITIONAL_TYPES
                        && qualifier.local == "MimeType"
                    {
                        mime_type = mime_type.or(value);
                    }
                }
            }
        }
        Ok(Pointer {
            location: self.text(pointer, "TSLLocation")?,
            certificates,
            territory,
            mime_type,
        })
    }

    fn provider(
        &self,
        provider: &Element,
        services: &mut usize,
    ) -> Result<Provider, TrustedListError> {
        let information = self.required(provider, "TSPInformation")?;
        let trade_name = match self.first(information, "TSPTradeName") {
            Some(_) => Some(self.name(information, "TSPTradeName")?),
            None => None,
        };
        let mut read = Vec::new();
        for service in self
            .first(provider, "TSPServices")
            .map(|list| self.all(list, "TSPService"))
            .unwrap_or_default()
        {
            read.push(self.service(service, services)?);
        }
        Ok(Provider {
            name: self.name(information, "TSPName")?,
            trade_name,
            services: read,
        })
    }

    fn service(&self, service: &Element, count: &mut usize) -> Result<Service, TrustedListError> {
        let information = self.required(service, "ServiceInformation")?;
        let current = self.state(information, count)?;
        let mut history = Vec::new();
        if let Some(list) = self.first(service, "ServiceHistory") {
            for instance in self.all(list, "ServiceHistoryInstance") {
                history.push(self.state(instance, count)?);
            }
        }
        Ok(Service { current, history })
    }

    /// Clause 5.5's fields, or clause 5.6's, which Annex C's schema spells with the same elements.
    fn state(
        &self,
        element: &Element,
        count: &mut usize,
    ) -> Result<ServiceState, TrustedListError> {
        *count = count.saturating_add(1);
        if *count > MAX_SERVICES {
            return Err(TrustedListError::TooManyEntries);
        }
        let identity = self.required(element, "ServiceDigitalIdentity")?;
        let mut certificates = Vec::new();
        for digital in self.all(identity, "DigitalId") {
            for certificate in self.all(digital, "X509Certificate") {
                certificates.push(
                    base64(&certificate.text)
                        .ok_or(TrustedListError::Malformed("X509Certificate"))?,
                );
            }
        }
        let mut additional_information = Vec::new();
        if let Some(extensions) = self.first(element, "ServiceInformationExtensions") {
            for extension in self.all(extensions, "Extension") {
                for information in self.all(extension, "AdditionalServiceInformation") {
                    additional_information.push(self.text(information, "URI")?);
                }
            }
        }
        Ok(ServiceState {
            type_identifier: self.text(element, "ServiceTypeIdentifier")?,
            name: self.name(element, "ServiceName")?,
            certificates,
            status: self.text(element, "ServiceStatus")?,
            since: self.instant(element, "StatusStartingTime")?,
            additional_information,
        })
    }
}

#[cfg(test)]
pub(crate) mod tests;
