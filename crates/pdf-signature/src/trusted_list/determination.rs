//! ETSI TS 119 615 clause 4.4: whether a Member State's trusted list confirms a certificate as an
//! EU qualified certificate at an instant, and for which purpose.
//!
//! The procedure runs clause 4.3's search ([`super::matching`]) on the list of the territory the
//! certificate's issuer names (PRO-4.4.4-01 to -03), checks that the issuer is the provider the
//! list names (PRO-4.4.4-06), and then decides three things separately — a qualified certificate
//! for electronic signatures, for electronic seals, for web site authentication — each by a table
//! whose row is what the certificate's EN 319 412-5 statements claim and whose column is what the
//! qualifiers of the service's `Qualifications` extensions ([`super::qualifications`]) add
//! (Tables 1 to 3). Table 4 then refuses two purposes at once, and the whole is run a second time
//! at the certificate's `notBefore`, whose answer has to be the same (PRO-4.4.4-34 to -36). A
//! certificate issued under Directive 1999/93/EC, or an instant before 2016-06-30T22:00:00Z, takes
//! PRO-4.4.4-33's older path and Table 5. The ETSI texts are cited by clause and paraphrased, never
//! quoted (ADR 1085).
//!
//! # What the answer is not
//!
//! **An indication about the certificate, never a level of the signature.** `QC_For_eSig` says
//! the list confirms the signer's certificate as qualified for electronic signatures; whether the
//! signature is a qualified one also needs clause 4.5's device determination and ETSI TS 119 172-4
//! REQ-4.4.2-06's join, which this module does not perform (ADR 2835, ADR 2864).
//!
//! # Readings the clause's text forces
//!
//! - PRO-4.4.4-18A c) sets `CHECK_1` inside the electronic seal branch, and PRO-4.4.4-28 b) sets
//!   `CHECK_3` to a sub-status value. Each sentence's siblings (PRO-4.4.4-10A, -12, -20) and
//!   NOTE 4 and NOTE 7 — which give each check only its table's values — say the check of the
//!   branch takes the branch's indeterminate value and the warning goes to `QC-Sub-Status`; that
//!   is what runs here.
//! - PRO-4.4.4-33 f) 3) stops the process with `QC-Status` passed and only `CHECK_1` set; the
//!   three checks it holds by then are returned as `QC-Results`, as step m) of the same
//!   requirement would have.
//! - PRO-4.4.4-06 b) leaves the strategy to the processor: the issuer's name matching the subject
//!   of a certificate of the service's digital identity (EXAMPLE 1 ii)), else its `commonName`
//!   matching a TSP name or trade name (EXAMPLE 1 i)), each by RFC 5280 section 7.1.
//! - Clause 4.3's `CC` input names the Member State's list; a host that supplied no list of that
//!   territory has not supplied the list the procedure reads, and that is said as itself
//!   ([`Qualification::NoListForTerritory`]) rather than as `Not_Qualified`.

use super::qualifications::{
    Examined, NOT_QUALIFIED, QC_FOR_ESEAL, QC_FOR_ESIG, QC_FOR_LEGAL_PERSON, QC_FOR_WSA,
    QC_STATEMENT,
};
use super::{CA_QC, FOR_TYPES, Fault, GRANTED, Matching, ServiceState, Status, TrustedList};
use crate::qc_statement::{QcType, Stated};
use crate::signature::Signature;
use crate::trust::names::{self, Match};
use crate::x509::{self, Certificate, Instant};

/// 2016-06-30T22:00:00Z, the instant PRO-4.4.4-07 divides Regulation (EU) No 910/2014's lists
/// from Directive 1999/93/EC's.
pub const REGULATION_APPLIES: Instant = Instant::from_unix_seconds(1_467_324_000);

/// TS 119 612 clause 5.5.4's status of a qualified service that was withdrawn.
pub const WITHDRAWN: &str = "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/withdrawn";

/// The four statuses of the Directive's lists PRO-4.4.4-33 f) 1) reads as an end of supervision.
pub const SUPERVISION_ENDED: [&str; 4] = [
    "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/supervisionceased",
    "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/supervisionrevoked",
    "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/accreditationceased",
    "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/accreditationrevoked",
];

/// ETSI EN 319 411-2's `qcp-public-with-sscd` (QCP+), which PRO-4.4.4-33 i) asks for.
pub const QCP_PLUS: &str = "0.4.0.1456.1.1";
/// ETSI EN 319 411-2's `qcp-public` (QCP).
pub const QCP: &str = "0.4.0.1456.1.2";

/// `countryName`, ITU-T X.520's `id-at-countryName`.
const COUNTRY_NAME: &str = "2.5.4.6";
/// `organizationName`.
const ORGANIZATION_NAME: &str = "2.5.4.10";
/// `commonName`.
const COMMON_NAME: &str = "2.5.4.3";

/// Clause 4.4.3's `QC-Results` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Indication {
    /// a) `Not_Qualified`.
    NotQualified,
    /// b) `Not_Qualified_For_eSig`.
    NotQualifiedForESig,
    /// c) `Not_Qualified_For_eSeal`.
    NotQualifiedForESeal,
    /// d) `Not_QWAC`.
    NotQwac,
    /// e) `QC_For_eSig`.
    QcForESig,
    /// f) `QC_For_eSeal`.
    QcForESeal,
    /// g) `QWAC`.
    Qwac,
    /// h) `INDET_QC_For_eSig`.
    IndetQcForESig,
    /// i) `INDET_QC_For_eSeal`.
    IndetQcForESeal,
    /// j) `INDET_QWAC`.
    IndetQwac,
    /// k) `INDETERMINATE`.
    Indeterminate,
}

impl Indication {
    /// The value as clause 4.4.3 spells it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::NotQualified => "Not_Qualified",
            Self::NotQualifiedForESig => "Not_Qualified_For_eSig",
            Self::NotQualifiedForESeal => "Not_Qualified_For_eSeal",
            Self::NotQwac => "Not_QWAC",
            Self::QcForESig => "QC_For_eSig",
            Self::QcForESeal => "QC_For_eSeal",
            Self::Qwac => "QWAC",
            Self::IndetQcForESig => "INDET_QC_For_eSig",
            Self::IndetQcForESeal => "INDET_QC_For_eSeal",
            Self::IndetQwac => "INDET_QWAC",
            Self::Indeterminate => "INDETERMINATE",
        }
    }

    /// The purpose a qualified or indeterminate value is about, for Table 4.
    const fn purpose(self) -> Option<(Purpose, Kind)> {
        match self {
            Self::QcForESig => Some((Purpose::ESig, Kind::Qualified)),
            Self::QcForESeal => Some((Purpose::ESeal, Kind::Qualified)),
            Self::Qwac => Some((Purpose::Wsa, Kind::Qualified)),
            Self::IndetQcForESig => Some((Purpose::ESig, Kind::Indeterminate)),
            Self::IndetQcForESeal => Some((Purpose::ESeal, Kind::Indeterminate)),
            Self::IndetQwac => Some((Purpose::Wsa, Kind::Indeterminate)),
            _ => None,
        }
    }
}

/// Clause 4.4.3's `QC-Status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QcStatus {
    /// `PROCESS_PASSED`.
    Passed,
    /// `PROCESS_PASSED_WITH_WARNING`.
    PassedWithWarning,
    /// `PROCESS_FAILED`.
    Failed,
}

/// One `QC-Sub-Status` value: the clause's own indications, named by them, and three of this
/// reader's, each named as not the clause's.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SubStatus {
    /// PRO-4.4.4-04 b): clause 4.3's `SI-Status` failed, with its `SI-Sub-Status`.
    Search(Fault),
    /// PRO-4.4.4-05 b): `No_confirmation_found_in_EUMSTL_CC`.
    NoConfirmationFound,
    /// PRO-4.4.4-06: `ERROR_TSP_NAME_INCONSISTENCY_BETWEEN_CERT_AND_TL`.
    TspNameInconsistency,
    /// PRO-4.4.4-10A, -18A, -26A: `ERROR_Service_Status_ Non-compliance_with_TS119612`.
    ServiceStatusNonCompliance,
    /// PRO-4.4.4-12, -20, -28: `WARNING_T1_TL_Inconsistency_in_applying_qualifiers` and its
    /// siblings, by the purpose's table.
    InconsistentQualifiers(Purpose),
    /// PRO-4.4.4-33 h) ii): `ERROR_T1_TL_Inconsistency_in_applying_qualifiers`.
    InconsistentQualifiersError,
    /// PRO-4.4.4-14, -22, -30:
    /// `WARNING_CERT_Inconsistency_in_QcType_qualifiers_Non-compliance_with_EN319412-5`.
    QcTypeInconsistency,
    /// PRO-4.4.4-15 d), -23 d), -31 d): `WARNING_T1_Not_Enough_Info_on_QC_Type` and its siblings.
    NotEnoughInfoOnQcType(Purpose),
    /// PRO-4.4.4-32: a two-by-two combination Table 4 marks an error (`true`) or a warning.
    Combination {
        /// The first value of the pair.
        first: Indication,
        /// The second.
        second: Indication,
        /// Whether Table 4 marks it an error.
        error: bool,
    },
    /// PRO-4.4.4-33 c): `WARNING_TL-SERVICE-ENTRY-SDI_DUPLICATION`.
    SdiDuplication,
    /// PRO-4.4.4-33 d): `ERROR_TL-SERVICE-ENTRY-SDI_DUPLICATION_STATUS_CONFLICT`.
    SdiDuplicationConflict,
    /// PRO-4.4.4-35: the run at the certificate's `notBefore` failed.
    FailedAtIssue,
    /// PRO-4.4.4-36 a): the run at the certificate's `notBefore` returned these results instead.
    ResultsDifferAtIssue(Vec<Indication>),
    /// This reader's: a critical `Qualifications` extension holds criteria it cannot evaluate,
    /// so TS 119 612 clause 5.5.9.2.0 discards the certificate.
    CriticalCriteriaUndecided,
    /// This reader's: a non-critical `Qualifications` extension's element whose criteria it cannot
    /// evaluate was set aside.
    CriteriaUndecided,
    /// This reader's: the certificate's EN 319 412-5 statements do not read, so no row of the
    /// tables can be chosen.
    StatementsUnreadable,
    /// This reader's: the issuer's name and the list's provider names could not be compared
    /// (RFC 5280 section 7.1 on text this tree does not prepare).
    TspNamesUndecided,
}

impl SubStatus {
    /// The value as clause 4.4.4 spells it, or a phrase saying it is this reader's.
    #[must_use]
    pub fn name(&self) -> String {
        match self {
            Self::Search(fault) => format!("clause 4.3 failed ({fault:?})"),
            Self::NoConfirmationFound => "No_confirmation_found_in_EUMSTL_CC".to_owned(),
            Self::TspNameInconsistency => {
                "ERROR_TSP_NAME_INCONSISTENCY_BETWEEN_CERT_AND_TL".to_owned()
            }
            Self::ServiceStatusNonCompliance => {
                "ERROR_Service_Status_ Non-compliance_with_TS119612".to_owned()
            }
            Self::InconsistentQualifiers(purpose) => format!(
                "WARNING_T{}_TL_Inconsistency_in_applying_qualifiers",
                purpose.table()
            ),
            Self::InconsistentQualifiersError => {
                "ERROR_T1_TL_Inconsistency_in_applying_qualifiers".to_owned()
            }
            Self::QcTypeInconsistency => {
                "WARNING_CERT_Inconsistency_in_QcType_qualifiers_Non-compliance_with_EN319412-5"
                    .to_owned()
            }
            Self::NotEnoughInfoOnQcType(purpose) => {
                format!("WARNING_T{}_Not_Enough_Info_on_QC_Type", purpose.table())
            }
            Self::Combination {
                first,
                second,
                error,
            } => format!(
                "Table 4 {} for {} with {}",
                if *error { "error" } else { "warning" },
                first.name(),
                second.name()
            ),
            Self::SdiDuplication => "WARNING_TL-SERVICE-ENTRY-SDI_DUPLICATION".to_owned(),
            Self::SdiDuplicationConflict => {
                "ERROR_TL-SERVICE-ENTRY-SDI_DUPLICATION_STATUS_CONFLICT".to_owned()
            }
            Self::FailedAtIssue => "the run at the certificate's notBefore failed".to_owned(),
            Self::ResultsDifferAtIssue(results) => format!(
                "the run at the certificate's notBefore returned {}",
                names_of(results)
            ),
            Self::CriticalCriteriaUndecided => {
                "a critical Qualifications extension's criteria cannot be evaluated by this reader"
                    .to_owned()
            }
            Self::CriteriaUndecided => {
                "a Qualifications element whose criteria this reader cannot evaluate was set aside"
                    .to_owned()
            }
            Self::StatementsUnreadable => {
                "the certificate's ETSI EN 319 412-5 statements do not read".to_owned()
            }
            Self::TspNamesUndecided => {
                "the issuer's name could not be compared with the list's provider names".to_owned()
            }
        }
    }

    /// Whether the value is a warning, which PRO-4.4.4-36 b) carries into the status.
    #[must_use]
    pub const fn is_warning(&self) -> bool {
        matches!(
            self,
            Self::InconsistentQualifiers(_)
                | Self::QcTypeInconsistency
                | Self::NotEnoughInfoOnQcType(_)
                | Self::Combination { error: false, .. }
                | Self::SdiDuplication
                | Self::CriteriaUndecided
        )
    }
}

/// The three purposes, and the table each is decided by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Purpose {
    /// Electronic signatures: Table 1, `CHECK_1`.
    ESig,
    /// Electronic seals: Table 2, `CHECK_2`.
    ESeal,
    /// Web site authentication: Table 3, `CHECK_3`.
    Wsa,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Qualified,
    Indeterminate,
}

impl Purpose {
    const fn table(self) -> u8 {
        match self {
            Self::ESig => 1,
            Self::ESeal => 2,
            Self::Wsa => 3,
        }
    }

    /// Index into [`FOR_TYPES`], the `additionalServiceInformation` value the purpose's branch
    /// filters by, and into clause 4.3's duplication faults.
    const fn index(self) -> usize {
        match self {
            Self::ESig => 0,
            Self::ESeal => 1,
            Self::Wsa => 2,
        }
    }

    const fn qualified(self) -> Indication {
        match self {
            Self::ESig => Indication::QcForESig,
            Self::ESeal => Indication::QcForESeal,
            Self::Wsa => Indication::Qwac,
        }
    }

    const fn not_qualified(self) -> Indication {
        match self {
            Self::ESig => Indication::NotQualifiedForESig,
            Self::ESeal => Indication::NotQualifiedForESeal,
            Self::Wsa => Indication::NotQwac,
        }
    }

    const fn indeterminate(self) -> Indication {
        match self {
            Self::ESig => Indication::IndetQcForESig,
            Self::ESeal => Indication::IndetQcForESeal,
            Self::Wsa => Indication::IndetQwac,
        }
    }

    /// The qualifier PRO-4.4.4-15 a), -23 a) and -31 a) look for beside `NotQualified` and
    /// `QCStatement`.
    const fn qualifier(self) -> &'static str {
        match self {
            Self::ESig => QC_FOR_ESIG,
            Self::ESeal => QC_FOR_ESEAL,
            Self::Wsa => QC_FOR_WSA,
        }
    }

    /// PRO-4.4.4-12 a), -20 b) and -28 b): the qualifiers and combinations that make the branch
    /// indeterminate, item by item as each requirement lists them.
    const fn inconsistent(self) -> &'static [&'static [&'static str]] {
        match self {
            Self::ESig => &[
                &[QC_FOR_ESEAL],
                &[QC_FOR_WSA],
                &[NOT_QUALIFIED, QC_STATEMENT],
                &[QC_FOR_ESIG, QC_FOR_ESEAL],
                &[QC_FOR_ESIG, QC_FOR_WSA],
                &[QC_FOR_ESEAL, QC_FOR_WSA],
                &[QC_FOR_LEGAL_PERSON],
            ],
            Self::ESeal => &[
                &[QC_FOR_ESIG],
                &[QC_FOR_WSA],
                &[NOT_QUALIFIED, QC_STATEMENT],
                &[QC_FOR_ESIG, QC_FOR_ESEAL],
                &[QC_FOR_ESIG, QC_FOR_WSA],
                &[QC_FOR_ESEAL, QC_FOR_WSA],
            ],
            Self::Wsa => &[
                &[QC_FOR_ESIG],
                &[QC_FOR_ESEAL],
                &[NOT_QUALIFIED, QC_STATEMENT],
                &[QC_FOR_ESIG, QC_FOR_ESEAL],
                &[QC_FOR_ESIG, QC_FOR_WSA],
                &[QC_FOR_ESEAL, QC_FOR_WSA],
            ],
        }
    }

    /// Tables 1 to 3, rows 1 to 15 by columns 1 to 5, as the clause prints them: `Q` the
    /// purpose's qualified value, `N` its not-qualified value, `I` its indeterminate value.
    const fn table_rows(self) -> [&'static str; 15] {
        match self {
            Self::ESig => [
                "QNQQQ", "NNNQQ", "NNNQQ", "INIQQ", "INIQQ", "NNNQQ", "INIQQ", "NNINQ", "NNQNQ",
                "NNNNQ", "NNNNQ", "NNINQ", "NNINQ", "NNNNQ", "NNINQ",
            ],
            Self::ESeal => [
                "NNNQQ", "QNQQQ", "NNNQQ", "INIQQ", "NNNQQ", "INIQQ", "INIQQ", "NNINQ", "NNNNQ",
                "NNQNQ", "NNNNQ", "NNINQ", "NNNNQ", "NNINQ", "NNINQ",
            ],
            Self::Wsa => [
                "NNNQQ", "NNNQQ", "QNQQQ", "NNNQQ", "INIQQ", "INIQQ", "INIQQ", "NNINQ", "NNNNQ",
                "NNNNQ", "NNQNQ", "NNNNQ", "NNINQ", "NNINQ", "NNINQ",
            ],
        }
    }

    /// The cell of the purpose's table at `row` (1 to 15) and `column` (1 to 5).
    #[must_use]
    pub fn cell(self, row: usize, column: usize) -> Option<Indication> {
        let rows = self.table_rows();
        let letter = rows
            .get(row.checked_sub(1)?)?
            .as_bytes()
            .get(column.checked_sub(1)?)?;
        match letter {
            b'Q' => Some(self.qualified()),
            b'N' => Some(self.not_qualified()),
            b'I' => Some(self.indeterminate()),
            _ => None,
        }
    }
}

/// Tables 1 to 3's row for what a certificate's statements claim: rows 1 to 7 with
/// `QcCompliance` and rows 8 to 15 without, each block ordered no type (merged into row 1 with
/// compliance, as NOTE 3 says), type 1, type 2, type 3, types 1 and 2, 1 and 3, 2 and 3, all three.
#[must_use]
pub fn row(compliance: bool, types: &[QcType]) -> usize {
    // The block's offset by the set of the three types stated, indexed by the set as bits.
    const ROW_OF_SET: [usize; 8] = [0, 1, 2, 4, 3, 5, 6, 7];
    let set = usize::from(types.contains(&QcType::Esign))
        | usize::from(types.contains(&QcType::Eseal)) << 1
        | usize::from(types.contains(&QcType::Web)) << 2;
    // The index is three bits, so it lies inside the eight-entry table.
    let offset = ROW_OF_SET.get(set).copied().unwrap_or(0);
    if compliance {
        offset.max(1)
    } else {
        offset.saturating_add(8)
    }
}

/// Tables 1 to 3's column for the qualifiers applying to the certificate: column 2 wherever
/// `NotQualified` applies (with or without the purpose's qualifier), else by `QCStatement` and the
/// purpose's qualifier.
#[must_use]
pub fn column(not_qualified: bool, statement: bool, purpose_qualifier: bool) -> usize {
    match (not_qualified, statement, purpose_qualifier) {
        (true, _, _) => 2,
        (false, false, false) => 1,
        (false, true, false) => 3,
        (false, false, true) => 4,
        (false, true, true) => 5,
    }
}

/// Clause 4.4's three outputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Determination {
    /// `QC-Status`.
    pub status: QcStatus,
    /// `QC-Sub-Status`, each value once, in the order the processing added them.
    pub sub_status: Vec<SubStatus>,
    /// `QC-Results`.
    pub results: Vec<Indication>,
    /// `CHECK_1_SET-OF_QE`, `CHECK_2_SET-OF_QE` and `CHECK_3_SET-OF_QE` (clause 4.4.3), as clause
    /// 4.5 reads them: the qualifiers of each element that identified the certificate, each with
    /// whether its extension was critical. Empty where the run did not reach the set.
    pub sets_of_qe: [Vec<Applied>; 3],
}

/// One qualifier of a qualification element that identified the certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    /// The qualifier URI (TS 119 612 clause 5.5.9.2.3).
    pub uri: String,
    /// Whether the `Qualifications` extension it came from is marked critical.
    pub critical: bool,
}

impl Determination {
    fn add(&mut self, value: SubStatus) {
        if !self.sub_status.contains(&value) {
            self.sub_status.push(value);
        }
    }
}

/// What clause 4.4 could be asked about a signer's certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Qualification {
    /// The procedure ran on the list of `territory`.
    Determined {
        /// PRO-4.4.4-01's `CC`.
        territory: String,
        /// The list's TS 119 612 clause 5.3.2 sequence number.
        sequence_number: u64,
        /// The outputs.
        determination: Determination,
        /// Clause 4.5's outputs, from these.
        qscd: qscd::Qscd,
    },
    /// No list of the territory PRO-4.4.4-01 names was supplied, so the list the procedure reads
    /// is not there to read.
    NoListForTerritory(String),
    /// The issuer's name states no `countryName`, so PRO-4.4.4-01 names no list.
    IssuerStatesNoCountry,
    /// The signer's certificate does not read, or states no validity, so neither `CERT` nor
    /// PRO-4.4.4-34's second instant exists.
    NoCertificate,
}

/// Clause 4.4 for a signature's signing certificate at `at` — ETSI TS 119 172-4 REQ-4.4.2-01
/// passes the best signature time — over the lists a host supplied, with the certificates the
/// signature carries as the pool clause 4.3 builds paths through.
#[must_use]
pub fn qualification(lists: &[TrustedList], signature: &Signature, at: Instant) -> Qualification {
    let Some(der) = signature.signer_certificate() else {
        return Qualification::NoCertificate;
    };
    let Ok(certificate) = x509::parse(&der) else {
        return Qualification::NoCertificate;
    };
    let carried = signature.carried_certificates();
    let pool: Vec<Certificate<'_>> = carried
        .iter()
        .filter_map(|der| x509::parse(der).ok())
        .filter(|candidate| candidate.tbs != certificate.tbs)
        .collect();
    determine(lists, &certificate, &pool, at)
}

/// Clause 4.4 for `certificate` at `at`, PRO-4.4.4-01 to -36.
#[must_use]
pub fn determine(
    lists: &[TrustedList],
    certificate: &Certificate<'_>,
    pool: &[Certificate<'_>],
    at: Instant,
) -> Qualification {
    let Some(validity) = certificate.validity else {
        return Qualification::NoCertificate;
    };
    let Some(territory) = territory_of(certificate) else {
        return Qualification::IssuerStatesNoCountry;
    };
    // A host may hold two issues of one territory's list; the later one is the list.
    let Some(list) = lists
        .iter()
        .filter(|list| list.territory == territory)
        .max_by_key(|list| list.sequence_number)
    else {
        return Qualification::NoListForTerritory(territory);
    };
    let asked = Asked {
        list,
        certificate,
        pool,
        examined: Examined::of(certificate),
        statements: Stated::of(certificate),
        not_before: validity.not_before,
    };
    let (mut first, stopped) = asked.run(at);
    if !stopped {
        // PRO-4.4.4-34: the same procedure at the certificate's issue, up to PRO-4.4.4-32 or -33.
        let (second, _) = asked.run(validity.not_before);
        if second.status == QcStatus::Failed {
            // PRO-4.4.4-35.
            first.status = QcStatus::Failed;
            first.add(SubStatus::FailedAtIssue);
            for value in second.sub_status {
                first.add(value);
            }
        } else if !same_set(&first.results, &second.results) {
            // PRO-4.4.4-36 a).
            first.status = QcStatus::Failed;
            first.add(SubStatus::ResultsDifferAtIssue(second.results));
        } else if second.sub_status.iter().any(SubStatus::is_warning) {
            // PRO-4.4.4-36 b).
            first.status = QcStatus::PassedWithWarning;
            for value in second.sub_status {
                first.add(value);
            }
        }
    }
    // Clause 4.5 at the same instant, over this run's outputs (PRO-4.5.4-01).
    let qscd = qscd::determine(&first, &asked.statements, &asked.examined, at);
    Qualification::Determined {
        territory,
        sequence_number: list.sequence_number,
        determination: first,
        qscd,
    }
}

/// PRO-4.4.4-01: the issuer's `countryName`, in capitals, `GB` written `UK` and `GR` written `EL`.
fn territory_of(certificate: &Certificate<'_>) -> Option<String> {
    let attributes = names::attributes_of(certificate.issuer).ok()?;
    let country = attributes
        .iter()
        .find(|attribute| attribute.kind == COUNTRY_NAME)?
        .text
        .as_deref()?
        .trim()
        .to_ascii_uppercase();
    Some(match country.as_str() {
        "GB" => "UK".to_owned(),
        "GR" => "EL".to_owned(),
        _ => country,
    })
}

fn same_set(left: &[Indication], right: &[Indication]) -> bool {
    left.iter().all(|value| right.contains(value)) && right.iter().all(|value| left.contains(value))
}

fn names_of(results: &[Indication]) -> String {
    if results.is_empty() {
        return "nothing".to_owned();
    }
    results
        .iter()
        .map(|value| value.name())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The inputs one run of the procedure shares with the other.
struct Asked<'a, 'c> {
    list: &'a TrustedList,
    certificate: &'a Certificate<'c>,
    pool: &'a [Certificate<'c>],
    examined: Examined,
    statements: Stated,
    not_before: Instant,
}

/// Why a branch ended the whole process rather than setting its check.
enum Stop {
    /// TS 119 612 clause 5.5.9.2.0: a critical extension's criteria cannot be evaluated.
    Discard,
}

impl Asked<'_, '_> {
    /// PRO-4.4.4-01 to -33 at `date_time`; the flag says the process stopped, so that PRO-4.4.4-34
    /// is not reached.
    fn run(&self, date_time: Instant) -> (Determination, bool) {
        let mut out = Determination {
            status: QcStatus::Passed,
            sub_status: Vec::new(),
            results: Vec::new(),
            sets_of_qe: [Vec::new(), Vec::new(), Vec::new()],
        };
        // PRO-4.4.4-02 and -03.
        let found = super::matching(self.list, self.certificate, self.pool, CA_QC, date_time);
        // PRO-4.4.4-04.
        if found.status == Status::Failed {
            out.status = QcStatus::Failed;
            out.sub_status = found
                .faults
                .iter()
                .copied()
                .map(SubStatus::Search)
                .collect();
            return (out, true);
        }
        // PRO-4.4.4-05.
        if found.results.is_empty() {
            out.add(SubStatus::NoConfirmationFound);
            out.results = vec![Indication::NotQualified];
            return (out, true);
        }
        // PRO-4.4.4-06.
        if let Err(value) = self.issuer_is_provider(&found) {
            out.status = QcStatus::Failed;
            out.results = vec![Indication::Indeterminate];
            out.add(value);
            return (out, true);
        }
        let states: Vec<&ServiceState> = found
            .results
            .iter()
            .filter_map(|matched| matched.at_date_time)
            .collect();
        // PRO-4.4.4-07.
        if date_time < REGULATION_APPLIES || self.not_before < REGULATION_APPLIES {
            return self.directive(date_time, &states, out);
        }
        let mut checks = Vec::new();
        for purpose in [Purpose::ESig, Purpose::ESeal, Purpose::Wsa] {
            match self.check(purpose, &found, &states, &mut out) {
                Ok(check) => checks.push(check),
                Err(Stop::Discard) => return discarded(out),
            }
        }
        // PRO-4.4.4-32.
        let mut warned = false;
        let mut failed = false;
        for (at, &first) in checks.iter().enumerate() {
            for &second in checks.iter().skip(at.saturating_add(1)) {
                match combination(first, second) {
                    Combined::Ok => {}
                    Combined::Warning => {
                        warned = true;
                        out.add(SubStatus::Combination {
                            first,
                            second,
                            error: false,
                        });
                    }
                    Combined::Error => {
                        failed = true;
                        out.add(SubStatus::Combination {
                            first,
                            second,
                            error: true,
                        });
                    }
                }
            }
        }
        if failed {
            out.status = QcStatus::Failed;
            return (out, true);
        }
        out.status = if warned {
            QcStatus::PassedWithWarning
        } else {
            QcStatus::Passed
        };
        out.results = checks;
        (out, false)
    }

    /// PRO-4.4.4-06: the issuer's `organizationName` is one of the provider's names, or, where it
    /// states none, this reader's strategy for b) (module documentation).
    fn issuer_is_provider(&self, found: &Matching<'_>) -> Result<(), SubStatus> {
        let attributes = names::attributes_of(self.certificate.issuer)
            .map_err(|_| SubStatus::TspNamesUndecided)?;
        let provider_names: Vec<&str> = found
            .results
            .iter()
            .flat_map(|matched| matched.provider.names.iter().map(String::as_str))
            .collect();
        let matches_a_name = |text: Option<&str>| -> Match {
            let Some(text) = text else {
                return Match::Undecided;
            };
            provider_names.iter().fold(Match::No, |best, name| {
                match (best, names::texts_match(text, name)) {
                    (Match::Yes, _) | (_, Match::Yes) => Match::Yes,
                    (Match::Undecided, _) | (_, Match::Undecided) => Match::Undecided,
                    _ => Match::No,
                }
            })
        };
        let organizations: Vec<Option<&str>> = attributes
            .iter()
            .filter(|attribute| attribute.kind == ORGANIZATION_NAME)
            .map(|attribute| attribute.text.as_deref())
            .collect();
        if !organizations.is_empty() {
            // a): every organisation the issuer names is one of the provider's names.
            for organization in organizations {
                match matches_a_name(organization) {
                    Match::Yes => {}
                    Match::No => return Err(SubStatus::TspNameInconsistency),
                    Match::Undecided => return Err(SubStatus::TspNamesUndecided),
                }
            }
            return Ok(());
        }
        // b), EXAMPLE 1 ii): the issuer is the subject of a certificate of the service's identity.
        let identity_subject = found.results.iter().any(|matched| {
            matched.service.current.certificates.iter().any(|der| {
                x509::parse(der).is_ok_and(|identity| {
                    names::names_match(self.certificate.issuer, identity.subject) == Match::Yes
                })
            })
        });
        if identity_subject {
            return Ok(());
        }
        // b), EXAMPLE 1 i): the issuer's common name is one of the provider's names.
        let common = attributes
            .iter()
            .filter(|attribute| attribute.kind == COMMON_NAME)
            .map(|attribute| matches_a_name(attribute.text.as_deref()))
            .any(|answer| answer == Match::Yes);
        if common {
            Ok(())
        } else {
            Err(SubStatus::TspNameInconsistency)
        }
    }

    /// PRO-4.4.4-08 to -15 for electronic signatures, -16 to -23 for seals and -24 to -31 for web
    /// site authentication: one check's value.
    fn check(
        &self,
        purpose: Purpose,
        found: &Matching<'_>,
        states: &[&ServiceState],
        out: &mut Determination,
    ) -> Result<Indication, Stop> {
        // PRO-4.4.4-09, -17, -25.
        if found
            .faults
            .contains(&Fault::DuplicationError(purpose.index()))
        {
            return Ok(purpose.indeterminate());
        }
        let for_type = FOR_TYPES.get(purpose.index()).copied().unwrap_or_default();
        let with: Vec<&ServiceState> = states
            .iter()
            .copied()
            .filter(|state| {
                state
                    .additional_information
                    .iter()
                    .any(|uri| uri == for_type)
            })
            .collect();
        // PRO-4.4.4-10, -18, -26.
        if with.is_empty() || with.iter().any(|state| state.status == WITHDRAWN) {
            return Ok(purpose.not_qualified());
        }
        // PRO-4.4.4-10A, -18A, -26A.
        if with
            .iter()
            .any(|state| state.status != GRANTED && state.status != WITHDRAWN)
        {
            out.add(SubStatus::ServiceStatusNonCompliance);
            return Ok(purpose.indeterminate());
        }
        // PRO-4.4.4-11 and -12 a), -19 and -20 a), -27 and -28 a).
        let qualifiers = self.applicable_qualifiers(&with, purpose.index(), out)?;
        let has = |uri: &str| qualifiers.iter().any(|held| held == uri);
        // PRO-4.4.4-12 a), -20 b), -28 b).
        if purpose
            .inconsistent()
            .iter()
            .any(|combination| combination.iter().all(|uri| has(uri)))
        {
            out.add(SubStatus::InconsistentQualifiers(purpose));
            return Ok(purpose.indeterminate());
        }
        // PRO-4.4.4-13 and -14, -21 and -22, -29 and -30.
        let (compliance, types) = match &self.statements {
            Stated::Read(read) => (read.compliance, read.types.clone().unwrap_or_default()),
            Stated::Unreadable(_) => {
                out.add(SubStatus::StatementsUnreadable);
                return Ok(purpose.indeterminate());
            }
            // No extension, and any standing a later build adds, claims nothing.
            _ => (false, Vec::new()),
        };
        if types.len() > 1 {
            out.add(SubStatus::QcTypeInconsistency);
        }
        // PRO-4.4.4-15, -23, -31.
        let row = row(compliance, &types);
        let column = column(
            has(NOT_QUALIFIED),
            has(QC_STATEMENT),
            has(purpose.qualifier()),
        );
        if row == 8 && column == 3 {
            out.add(SubStatus::NotEnoughInfoOnQcType(purpose));
        }
        Ok(purpose
            .cell(row, column)
            .unwrap_or_else(|| purpose.indeterminate()))
    }

    /// The qualifiers of every qualification element of `states`' `Qualifications` extensions
    /// whose criteria identify the certificate.
    ///
    /// The set is kept as `CHECK_n_SET-OF_QE` at `index`, for clause 4.5.
    fn applicable_qualifiers(
        &self,
        states: &[&ServiceState],
        index: usize,
        out: &mut Determination,
    ) -> Result<Vec<String>, Stop> {
        let mut applied = Vec::new();
        for qualifications in states.iter().flat_map(|state| &state.qualifications) {
            for element in &qualifications.elements {
                match element.criteria.identifies(&self.examined) {
                    Match::Yes => applied.extend(element.qualifiers.iter().map(|uri| Applied {
                        uri: uri.clone(),
                        critical: qualifications.critical,
                    })),
                    Match::No => {}
                    Match::Undecided if qualifications.critical => return Err(Stop::Discard),
                    Match::Undecided => out.add(SubStatus::CriteriaUndecided),
                }
            }
        }
        let qualifiers = applied.iter().map(|applied| applied.uri.clone()).collect();
        if let Some(set) = out.sets_of_qe.get_mut(index) {
            *set = applied;
        }
        Ok(qualifiers)
    }

    /// PRO-4.4.4-33: the certificate or the instant falls under Directive 1999/93/EC's lists.
    fn directive(
        &self,
        date_time: Instant,
        states: &[&ServiceState],
        mut out: Determination,
    ) -> (Determination, bool) {
        // a) and b).
        let check_2 = Indication::NotQualifiedForESeal;
        let check_3 = Indication::NotQwac;
        // c) and d).
        if states.len() >= 2 {
            if states
                .windows(2)
                .all(|pair| pair.first().map(|s| &s.status) == pair.last().map(|s| &s.status))
            {
                out.add(SubStatus::SdiDuplication);
            } else {
                out.add(SubStatus::SdiDuplicationConflict);
                out.status = QcStatus::Failed;
                return (out, true);
            }
        }
        // f).
        let ended = if date_time < REGULATION_APPLIES {
            states
                .iter()
                .any(|state| SUPERVISION_ENDED.contains(&state.status.as_str()))
        } else {
            let for_type = FOR_TYPES.first().copied().unwrap_or_default();
            let with: Vec<&&ServiceState> = states
                .iter()
                .filter(|state| {
                    state
                        .additional_information
                        .iter()
                        .any(|uri| uri == for_type)
                })
                .collect();
            with.is_empty() || with.iter().any(|state| state.status == WITHDRAWN)
        };
        if ended {
            out.results = vec![Indication::NotQualifiedForESig, check_2, check_3];
            return (out, true);
        }
        // g) and h).
        let qualifiers = match self.applicable_qualifiers(states, 0, &mut out) {
            Ok(qualifiers) => qualifiers,
            Err(Stop::Discard) => return discarded(out),
        };
        let has = |uri: &str| qualifiers.iter().any(|held| held == uri);
        if has(QC_FOR_ESEAL) || has(QC_FOR_WSA) || (has(NOT_QUALIFIED) && has(QC_STATEMENT)) {
            out.add(SubStatus::InconsistentQualifiersError);
            out.status = QcStatus::Failed;
            out.results = vec![Indication::IndetQcForESig, check_2, check_3];
            return (out, true);
        }
        // i) to k): Table 5.
        let claimed = match &self.statements {
            Stated::Read(read) => read.compliance,
            _ => false,
        } || matches!(&self.examined.policies, super::qualifications::Field::Read(held)
            if held.iter().any(|oid| oid == QCP || oid == QCP_PLUS));
        let check_1 = table_5(claimed, has(NOT_QUALIFIED), has(QC_STATEMENT));
        // l) and m).
        out.status = QcStatus::Passed;
        out.results = vec![check_1, check_2, check_3];
        (out, false)
    }
}

/// Table 5: rows 1 to 4 (a `QcCompliance` statement, QCP, QCP+, or any combination of them) read
/// alike, and row 5 is none of them; column 2 is `NotQualified`, column 3 `QCStatement`.
#[must_use]
pub const fn table_5(stated: bool, not_qualified: bool, statement: bool) -> Indication {
    if !not_qualified && (statement || stated) {
        Indication::QcForESig
    } else {
        Indication::NotQualifiedForESig
    }
}

/// TS 119 612 clause 5.5.9.2.0's discarded certificate, as this reader reports it.
fn discarded(mut out: Determination) -> (Determination, bool) {
    out.status = QcStatus::Failed;
    out.results = vec![Indication::Indeterminate];
    out.add(SubStatus::CriticalCriteriaUndecided);
    (out, true)
}

/// Table 4's three cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combined {
    /// `ok`.
    Ok,
    /// `warning`.
    Warning,
    /// `error`.
    Error,
}

/// Table 4 for two checks of different purposes: two qualified values are an error, an
/// indeterminate one beside anything is a warning, and every other pair is ok.
#[must_use]
pub fn combination(first: Indication, second: Indication) -> Combined {
    match (first.purpose(), second.purpose()) {
        (Some((a, Kind::Qualified)), Some((b, Kind::Qualified))) if a != b => Combined::Error,
        (Some((_, Kind::Indeterminate)), _) | (_, Some((_, Kind::Indeterminate))) => {
            Combined::Warning
        }
        _ => Combined::Ok,
    }
}

pub mod qscd;

#[cfg(test)]
mod tests;
