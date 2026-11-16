//! ETSI TS 119 615 clauses 4.6 and 4.7 — whether a token's authority is, by the trusted list of
//! its territory, an EU qualified trust service provider for a time-stamping service, and so
//! whether the token is an EU qualified time stamp — and the time-stamping services a host's lists
//! publish for a token's authority, which ETSI TS 119 172-4 REQ-4.5-01 d) i) asks the report to
//! say the absence of.
//!
//! # The two procedures
//!
//! - **Clause 4.6** ([`issuer_qualification`]) takes a certificate, an instant and a service type.
//!   Before the regulation's application date it answers `Not_Qualified` at once (PRO-4.6.4-01);
//!   otherwise it names the list by the certificate's *subject* `countryName` (PRO-4.6.4-02), runs
//!   clause 4.3 on it ([`super::matching`], PRO-4.6.4-03), and reads the services found: a failed
//!   search is `Indeterminate` (-04), none found is `Not_Qualified` (-05), found services
//!   disagreeing on their status at the instant are `Indeterminate` (-06), disagreeing on their key
//!   is a warning (-07), a subject whose `organizationName` is none of the provider's names is
//!   `Indeterminate` (-08), and otherwise the answer is `Qualified` where a status at the instant is
//!   `granted` and `Not_Qualified` where none is (-09).
//! - **Clause 4.7** ([`qualified_time_stamp`]) runs clause 4.6 for the `TSA/QTST` service type
//!   twice — at the instant the question is put (PRO-4.7.4-02) and at the token's `genTime`
//!   (-04) — keeps the first run's outputs (-03), fails where the second run failed (-05), and
//!   fails where the two runs' answers differ (-06).
//!
//! # Choices the clauses leave to the implementer, each written down as one (ADR 2878)
//!
//! - **A `CC` naming no supplied list.** Clause 4.3 reads "the" list of `CC`; where the host
//!   supplied none, there is no list to read and the procedure is not run
//!   ([`Answer::NoListForTerritory`]), exactly as clause 4.4's determination is not
//!   ([`super::determination::Qualification::NoListForTerritory`]). A subject stating no
//!   `countryName`, or one that is not a Member State's, is clause 4.3's own PRO-4.3.4-01 failure
//!   and runs to `Indeterminate` through PRO-4.6.4-04.
//! - **PRO-4.6.4-07's "public key values"** are the keys of each found service's clause 5.5.3
//!   certificates; a history instance carries none (TS 119 612 clause 5.6.3).
//! - **PRO-4.6.4-08's "matching"** is `trust::names::texts_match`, the comparison clause 4.4's
//!   PRO-4.4.4-06 makes of an issuer's name; every `organizationName` the subject states must
//!   match, a subject stating none matches nothing, and a comparison that cannot be decided is
//!   reported as such ([`SubStatus::TspNamesUndecided`]) rather than as a match.
//! - **PRO-4.7.4-06's "appropriate values"** are one: [`SubStatus::ResultsDifferAtGeneration`],
//!   carrying the second run's answer.
//!
//! The ETSI texts are cited by clause and paraphrased, never quoted (ADR 1085).

use super::determination::country_code;
use super::{Authentication, EU_TERRITORIES, Fault, Matching, Status, TrustedList, matching};
use crate::cms::SignedData;
use crate::trust::names::{self, Match};
use crate::x509::{self, Certificate, Instant};

/// TS 119 612 clause 5.5.1.1's service type of a qualified electronic time stamp generation
/// service, which PRO-4.7.4-01 passes to clause 4.6.
pub const TSA_QTST: &str = "http://uri.etsi.org/TrstSvc/Svctype/TSA/QTST";

/// TS 119 612 clause 5.5.1.1's service type of a time stamping generation service that is not
/// qualified — the second type ETSI TS 119 172-4 REQ-4.5-01 d) i) names.
pub const TSA: &str = "http://uri.etsi.org/TrstSvc/Svctype/TSA";

/// PRO-4.6.4-01's instant, `2016-06-30T22:00:00Z`: before it, no provider was qualified under the
/// regulation.
pub const REGULATION_APPLIES_FROM: Instant = Instant::from_unix_seconds(1_467_324_000);

/// TS 119 612 clause 5.5.4's `granted` status, which PRO-4.6.4-09 a) asks for.
const GRANTED: &str = super::GRANTED;

/// `organizationName`, ITU-T X.520's `id-at-organizationName`.
const ORGANIZATION_NAME: &str = "2.5.4.10";

/// Clause 4.6.3's `QTSTo-Results` and clause 4.7.3's `QTST-Results`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Indication {
    /// a) `Not_Qualified`.
    NotQualified,
    /// b) `Qualified`.
    Qualified,
    /// c) `Indeterminate`.
    Indeterminate,
}

impl Indication {
    /// The clause's own spelling.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::NotQualified => "Not_Qualified",
            Self::Qualified => "Qualified",
            Self::Indeterminate => "Indeterminate",
        }
    }
}

/// The values clauses 4.6.4 and 4.7.4 put in `QTSTo-Sub-Status` and `QTST-Sub-Status`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SubStatus {
    /// PRO-4.6.4-04 c) and PRO-4.7.4-05 b): a run's status `PROCESS_FAILED`, carried as a value.
    ProcessFailed,
    /// PRO-4.6.4-04 c): one of clause 4.3's `SI-Sub-Status` values.
    Matching(Fault),
    /// PRO-4.6.4-06 c): `ERROR_INCONSISTENCY_IN_TL_ON_TSTO-CERT_STATUS`.
    InconsistentStatus,
    /// PRO-4.6.4-07: `WARNING_DUPLICATION_OF_SERVICE_INFORMATION_IN_TL_REGARDING_TSTO-CERT`.
    DuplicatedServiceInformation,
    /// PRO-4.6.4-08 c): `ERROR_TSP_NAME_INCONSISTENCY_BETWEEN_TSTO-CERT_AND_TL`.
    TspNameInconsistency,
    /// PRO-4.6.4-08 undecided: an `organizationName` this reader cannot compare with the provider's
    /// names, which is not a match (module documentation).
    TspNamesUndecided,
    /// PRO-4.7.4-06 a) 2): the run at the token's `genTime` answered this instead.
    ResultsDifferAtGeneration(Indication),
}

impl SubStatus {
    /// The clause's own spelling of the value, or a description where the value is this reader's
    /// (module documentation).
    #[must_use]
    pub fn name(&self) -> String {
        match self {
            Self::ProcessFailed => "PROCESS_FAILED".to_owned(),
            Self::Matching(fault) => format!("{fault:?}"),
            Self::InconsistentStatus => "ERROR_INCONSISTENCY_IN_TL_ON_TSTO-CERT_STATUS".to_owned(),
            Self::DuplicatedServiceInformation => {
                "WARNING_DUPLICATION_OF_SERVICE_INFORMATION_IN_TL_REGARDING_TSTO-CERT".to_owned()
            }
            Self::TspNameInconsistency => {
                "ERROR_TSP_NAME_INCONSISTENCY_BETWEEN_TSTO-CERT_AND_TL".to_owned()
            }
            Self::TspNamesUndecided => {
                "the subject's organisation name could not be compared with the provider's names"
                    .to_owned()
            }
            Self::ResultsDifferAtGeneration(other) => {
                format!(
                    "at the token's generation time the answer was {}",
                    other.name()
                )
            }
        }
    }
}

/// Clause 4.6.3's outputs, which clause 4.7.3's repeat without `TSP-Name`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outputs {
    /// `QTSTo-Results` (`QTST-Results`).
    pub results: Indication,
    /// `QTSTo-Status` (`QTST-Status`).
    pub status: Status,
    /// `QTSTo-Sub-Status` (`QTST-Sub-Status`), in the order the processing added them.
    pub sub_status: Vec<SubStatus>,
    /// `CC`; `None` where PRO-4.6.4-01 stopped before PRO-4.6.4-02 set it, or the subject states no
    /// `countryName`.
    pub country: Option<String>,
    /// `TSP-Name`, where PRO-4.6.4-09 set it.
    pub tsp_name: Option<String>,
}

impl Outputs {
    fn stopped(results: Indication, status: Status, country: Option<String>) -> Self {
        Self {
            results,
            status,
            sub_status: Vec::new(),
            country,
            tsp_name: None,
        }
    }
}

/// What clause 4.6 or 4.7 could be asked.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Answer {
    /// The procedure ran, on the list of [`Outputs::country`] where it reached one.
    Determined(Outputs),
    /// `CC` names a Member State whose list the host did not supply, so the list clause 4.3 reads
    /// is not there to read.
    NoListForTerritory(String),
}

/// Clause 4.6 for `certificate` (`CERT`), at `date_time`, for `service_type` (`TLS-Sti`), over the
/// host's lists, with `pool` the certificates clause 4.3 builds paths through.
#[must_use]
pub fn issuer_qualification(
    lists: &[TrustedList],
    certificate: &Certificate<'_>,
    pool: &[Certificate<'_>],
    date_time: Instant,
    service_type: &str,
) -> Answer {
    // PRO-4.6.4-01.
    if date_time.unix_seconds() < REGULATION_APPLIES_FROM.unix_seconds() {
        return Answer::Determined(Outputs::stopped(
            Indication::NotQualified,
            Status::Passed,
            None,
        ));
    }
    // PRO-4.6.4-02.
    let country = country_code(certificate.subject);
    let found = match country.as_deref() {
        Some(territory) if EU_TERRITORIES.contains(&territory) => {
            // A host may hold two issues of one territory's list; the later one is the list.
            let Some(list) = lists
                .iter()
                .filter(|list| list.territory == territory)
                .max_by_key(|list| list.sequence_number)
            else {
                return Answer::NoListForTerritory(territory.to_owned());
            };
            // PRO-4.6.4-03.
            matching(list, certificate, pool, service_type, date_time)
        }
        // PRO-4.3.4-01: no code, or one that is not a Member State's, fails clause 4.3 itself.
        _ => Matching {
            status: Status::Failed,
            faults: vec![Fault::NotAnEuMemberState],
            results: Vec::new(),
        },
    };
    Answer::Determined(read_matching(certificate, &found, country))
}

/// PRO-4.6.4-04 to -09 over clause 4.3's outputs.
fn read_matching(
    certificate: &Certificate<'_>,
    found: &Matching<'_>,
    country: Option<String>,
) -> Outputs {
    // PRO-4.6.4-04.
    if found.status == Status::Failed {
        let mut out = Outputs::stopped(Indication::Indeterminate, Status::Failed, country);
        out.sub_status.push(SubStatus::ProcessFailed);
        out.sub_status
            .extend(found.faults.iter().copied().map(SubStatus::Matching));
        return out;
    }
    // PRO-4.6.4-05.
    if found.results.is_empty() {
        return Outputs::stopped(Indication::NotQualified, Status::Passed, country);
    }
    let statuses: Vec<Option<&str>> = found
        .results
        .iter()
        .map(|matched| matched.at_date_time.map(|state| state.status.as_str()))
        .collect();
    // PRO-4.6.4-06.
    if statuses.windows(2).any(|pair| pair.first() != pair.last()) {
        let mut out = Outputs::stopped(Indication::Indeterminate, Status::Failed, country);
        out.sub_status.push(SubStatus::InconsistentStatus);
        return out;
    }
    let mut sub_status = Vec::new();
    // PRO-4.6.4-07.
    let keys: Vec<Vec<Vec<u8>>> = found
        .results
        .iter()
        .map(|matched| {
            matched
                .service
                .current
                .certificates
                .iter()
                .filter_map(|der| x509::parse(der).ok())
                .map(|identity| identity.public_key_bits.to_vec())
                .collect()
        })
        .collect();
    if keys.windows(2).any(|pair| pair.first() != pair.last()) {
        sub_status.push(SubStatus::DuplicatedServiceInformation);
    }
    // PRO-4.6.4-08.
    if let Err(value) = subject_is_provider(certificate, found) {
        let mut out = Outputs::stopped(Indication::Indeterminate, Status::Failed, country);
        sub_status.push(value);
        out.sub_status = sub_status;
        return out;
    }
    // PRO-4.6.4-09.
    let granted = statuses.contains(&Some(GRANTED));
    Outputs {
        results: if granted {
            Indication::Qualified
        } else {
            Indication::NotQualified
        },
        status: Status::Passed,
        sub_status,
        country,
        tsp_name: found
            .results
            .first()
            .map(|matched| matched.provider.name.clone()),
    }
}

/// PRO-4.6.4-08: every `organizationName` of the subject is one of the found providers' names or
/// trade names (module documentation).
fn subject_is_provider(
    certificate: &Certificate<'_>,
    found: &Matching<'_>,
) -> Result<(), SubStatus> {
    let attributes =
        names::attributes_of(certificate.subject).map_err(|_| SubStatus::TspNamesUndecided)?;
    let organizations: Vec<Option<&str>> = attributes
        .iter()
        .filter(|attribute| attribute.kind == ORGANIZATION_NAME)
        .map(|attribute| attribute.text.as_deref())
        .collect();
    if organizations.is_empty() {
        return Err(SubStatus::TspNameInconsistency);
    }
    for organization in organizations {
        let Some(text) = organization else {
            return Err(SubStatus::TspNamesUndecided);
        };
        let answer = found
            .results
            .iter()
            .flat_map(|matched| matched.provider.names.iter())
            .fold(Match::No, |best, name| {
                match (best, names::texts_match(text, name)) {
                    (Match::Yes, _) | (_, Match::Yes) => Match::Yes,
                    (Match::Undecided, _) | (_, Match::Undecided) => Match::Undecided,
                    _ => Match::No,
                }
            });
        match answer {
            Match::Yes => {}
            Match::No => return Err(SubStatus::TspNameInconsistency),
            Match::Undecided => return Err(SubStatus::TspNamesUndecided),
        }
    }
    Ok(())
}

/// Clause 4.7 for a token whose authority's certificate is `certificate`, generated at
/// `gen_time`, asked about at `date_time`.
#[must_use]
pub fn qualified_time_stamp(
    lists: &[TrustedList],
    certificate: &Certificate<'_>,
    pool: &[Certificate<'_>],
    gen_time: Instant,
    date_time: Instant,
) -> Answer {
    // PRO-4.7.4-01 and -02.
    let first = match issuer_qualification(lists, certificate, pool, date_time, TSA_QTST) {
        Answer::Determined(outputs) => outputs,
        other @ Answer::NoListForTerritory(_) => return other,
    };
    // PRO-4.7.4-03: the first run's outputs are clause 4.7's, `TSP-Name` aside.
    let mut out = Outputs {
        tsp_name: None,
        ..first
    };
    // PRO-4.7.4-04.
    let second = match issuer_qualification(lists, certificate, pool, gen_time, TSA_QTST) {
        Answer::Determined(outputs) => outputs,
        other @ Answer::NoListForTerritory(_) => return other,
    };
    // PRO-4.7.4-05.
    if second.status == Status::Failed {
        out.status = Status::Failed;
        out.sub_status.push(SubStatus::ProcessFailed);
        out.sub_status.extend(second.sub_status);
        return Answer::Determined(out);
    }
    // PRO-4.7.4-06.
    if second.results != out.results {
        out.status = Status::Failed;
        out.sub_status
            .push(SubStatus::ResultsDifferAtGeneration(second.results));
    }
    Answer::Determined(out)
}

/// Clause 4.7 for a time-stamp token: its signer's certificate is `CERT`, and the certificates it
/// carries are the pool. `None` where the token names no certificate this reader can read.
#[must_use]
pub fn qualified_time_stamp_of(
    lists: &[TrustedList],
    token: &SignedData<'_>,
    gen_time: Instant,
    date_time: Instant,
) -> Option<Answer> {
    let (certificate, pool) = authority_of(token)?;
    Some(qualified_time_stamp(
        lists,
        &certificate,
        &pool,
        gen_time,
        date_time,
    ))
}

/// A token's signing certificate and the other certificates it carries, parsed.
fn authority_of<'a>(token: &SignedData<'a>) -> Option<(Certificate<'a>, Vec<Certificate<'a>>)> {
    let signer = crate::signature::signer_certificate(token)?;
    let certificate = x509::read(signer).ok()?;
    let pool = token
        .certificates
        .iter()
        .filter_map(|entry| x509::read(*entry).ok())
        .filter(|candidate| candidate.tbs != certificate.tbs)
        .collect();
    Some((certificate, pool))
}

/// A time-stamping service a host's list publishes for a token's authority: who lists it, and how
/// far the list was authenticated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedAuthority {
    /// The list's scheme territory (TS 119 612 clause 5.3.10).
    pub territory: String,
    /// The list's sequence number (clause 5.3.2).
    pub sequence_number: u64,
    /// The provider's name (clause 5.4.1).
    pub provider: String,
    /// The service's name (clause 5.5.2).
    pub service: String,
    /// [`TSA_QTST`] or [`TSA`].
    pub service_type: String,
    /// The service's status at the token's `genTime` (TS 119 615 PRO-4.3.4-03 b)), where it had one.
    pub status_at_generation: Option<String>,
    /// How far the list's authenticity was established.
    pub authentication: Authentication,
}

/// ETSI TS 119 172-4 REQ-4.5-01 d) i)'s question for one token: the services of types [`TSA_QTST`]
/// and [`TSA`] that TS 119 615 clause 4.3 returns, with `SI-Status` `PROCESS_PASSED`, for the
/// token's authority at its `genTime`, each with its digital identity as DER — the anchors a host's
/// lists supply for that token (ADR 2878).
///
/// Empty where no list publishes one — the absence d) i) asks the report to express — or where the
/// token names no certificate this reader can read.
#[must_use]
pub fn listed_authorities(
    lists: &[TrustedList],
    token: &SignedData<'_>,
    gen_time: Instant,
) -> Vec<(ListedAuthority, Vec<Vec<u8>>)> {
    let Some((certificate, pool)) = authority_of(token) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for list in lists {
        for service_type in [TSA_QTST, TSA] {
            let search = matching(list, &certificate, &pool, service_type, gen_time);
            if search.status != Status::Passed {
                continue;
            }
            for matched in search.results {
                found.push((
                    ListedAuthority {
                        territory: list.territory.clone(),
                        sequence_number: list.sequence_number,
                        provider: matched.provider.name.clone(),
                        service: matched.service.current.name.clone(),
                        service_type: matched.service.current.type_identifier.clone(),
                        status_at_generation: matched
                            .at_date_time
                            .map(|state| state.status.clone()),
                        authentication: list.authentication,
                    },
                    matched.service.current.certificates.clone(),
                ));
            }
        }
    }
    found
}

#[cfg(test)]
pub(crate) mod tests;
