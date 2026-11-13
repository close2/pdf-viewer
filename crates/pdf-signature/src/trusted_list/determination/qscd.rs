//! ETSI TS 119 615 clause 4.5: whether a Member State's trusted list confirms that a qualified
//! certificate's private key resided in a qualified signature or seal creation device (a QSCD) at
//! an instant.
//!
//! The procedure reads clause 4.4's outputs ([`super::Determination`]): where the certificate was
//! confirmed qualified for signatures or seals, the qualifiers of the elements that identified it
//! (`CHECK_1_SET-OF_QE` or `CHECK_2_SET-OF_QE`) and the certificate's own `QcSSCD` statement (ETSI
//! EN 319 412-5 clause 4.2.2) choose a cell of Table 7, or under Directive 1999/93/EC — an instant
//! strictly before 2016-06-30T22:00:00Z — of Table 6, where the certificate policy QCP+ stands
//! beside the statement. The ETSI texts are cited by clause and paraphrased, never quoted
//! (ADR 1085).
//!
//! **An indication about the certificate's key, never a level of the signature**: ETSI TS 119
//! 172-4 REQ-4.4.2-06 joins this with clause 4.4's answer and the signature's own validation, and
//! that join is not here (ADR 2835, ADR 2865).
//!
//! Two sentences of the clause cannot be taken literally and are read by their context:
//! PRO-4.5.4-03 4) names its points "(a).a." and "(a).c.", which are its own points 1) and 3); and
//! PRO-4.5.4-04 e) chooses Table 7's row by point c), the inconsistencies, where the row is the
//! `QcSSCD` statement point d) identifies.

use super::{Applied, Determination, Indication, QcStatus, SubStatus};
use crate::qc_statement::Stated;
use crate::trusted_list::qualifications::{
    Examined, Field, NOT_QUALIFIED, QC_FOR_ESEAL, QC_FOR_ESIG, QC_FOR_LEGAL_PERSON, QC_FOR_WSA,
    QC_STATEMENT,
};
use crate::x509::Instant;

/// TS 119 612 clause 5.5.9.2.3's qualifier: the key resides in an SSCD.
pub const QC_WITH_SSCD: &str = "http://uri.etsi.org/TrstSvc/TrustedList/SvcInfoExt/QCWithSSCD";
/// The key does not reside in an SSCD.
pub const QC_NO_SSCD: &str = "http://uri.etsi.org/TrstSvc/TrustedList/SvcInfoExt/QCNoSSCD";
/// The certificate says whether its key resides in an SSCD.
pub const QC_SSCD_STATUS_AS_IN_CERT: &str =
    "http://uri.etsi.org/TrstSvc/TrustedList/SvcInfoExt/QCSSCDStatusAsInCert";
/// The key resides in a QSCD.
pub const QC_WITH_QSCD: &str = "http://uri.etsi.org/TrstSvc/TrustedList/SvcInfoExt/QCWithQSCD";
/// The key does not reside in a QSCD.
pub const QC_NO_QSCD: &str = "http://uri.etsi.org/TrstSvc/TrustedList/SvcInfoExt/QCNoQSCD";
/// The certificate says whether its key resides in a QSCD.
pub const QC_QSCD_STATUS_AS_IN_CERT: &str =
    "http://uri.etsi.org/TrstSvc/TrustedList/SvcInfoExt/QCQSCDStatusAsInCert";
/// The key resides in a QSCD the qualified provider manages on the subject's behalf.
pub const QC_QSCD_MANAGED_ON_BEHALF: &str =
    "http://uri.etsi.org/TrstSvc/TrustedList/SvcInfoExt/QCQSCDManagedOnBehalf";

/// The qualifiers this processing knows (PRO-4.5.4-04 b)'s NOTE): every one TS 119 612 clause
/// 5.5.9.2.3 defines for a `CA/QC` service.
pub const KNOWN: [&str; 13] = [
    QC_WITH_SSCD,
    QC_NO_SSCD,
    QC_SSCD_STATUS_AS_IN_CERT,
    QC_WITH_QSCD,
    QC_NO_QSCD,
    QC_QSCD_STATUS_AS_IN_CERT,
    QC_QSCD_MANAGED_ON_BEHALF,
    QC_FOR_LEGAL_PERSON,
    QC_FOR_ESIG,
    QC_FOR_ESEAL,
    QC_FOR_WSA,
    NOT_QUALIFIED,
    QC_STATEMENT,
];

/// Clause 4.5.3's `QSCD-Results` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QscdIndication {
    /// a) `QSCD_YES`.
    Yes,
    /// b) `QSCD_NO`.
    No,
    /// c) `QSCD_INDETERMINATE`.
    Indeterminate,
}

impl QscdIndication {
    /// The value as clause 4.5.3 spells it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Yes => "QSCD_YES",
            Self::No => "QSCD_NO",
            Self::Indeterminate => "QSCD_INDETERMINATE",
        }
    }
}

/// One `QSCD-Sub-Status` value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum QscdSubStatus {
    /// PRO-4.5.4-02 b): clause 4.4 failed, with its sub-status.
    QualificationFailed(Vec<SubStatus>),
    /// PRO-4.5.4-03 a) 2): `WARNING_Inconsistency_in_applying_qualifiers_for_SSCD_status`.
    InconsistentSscd,
    /// PRO-4.5.4-04 b) 1): `ERROR_Unknown_critical_qualifiers_for_QSCD_status`.
    UnknownCriticalQualifiers,
    /// PRO-4.5.4-04 b) 2): `WARNING_Unknown_qualifiers_for_QSCD_status`.
    UnknownQualifiers,
    /// PRO-4.5.4-04 c): `WARNING_Inconsistency_in_applying_qualifiers_for_QSCD_status`.
    InconsistentQscd,
}

impl QscdSubStatus {
    /// The value as clause 4.5.4 spells it.
    #[must_use]
    pub fn name(&self) -> String {
        match self {
            Self::QualificationFailed(values) => format!(
                "clause 4.4 failed ({})",
                values
                    .iter()
                    .map(SubStatus::name)
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
            Self::InconsistentSscd => {
                "WARNING_Inconsistency_in_applying_qualifiers_for_SSCD_status".to_owned()
            }
            Self::UnknownCriticalQualifiers => {
                "ERROR_Unknown_critical_qualifiers_for_QSCD_status".to_owned()
            }
            Self::UnknownQualifiers => "WARNING_Unknown_qualifiers_for_QSCD_status".to_owned(),
            Self::InconsistentQscd => {
                "WARNING_Inconsistency_in_applying_qualifiers_for_QSCD_status".to_owned()
            }
        }
    }
}

/// Clause 4.5's three outputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Qscd {
    /// `QSCD-Status`.
    pub status: QcStatus,
    /// `QSCD-Sub-Status`.
    pub sub_status: Vec<QscdSubStatus>,
    /// `QSCD-Results`; `None` where the process stopped before setting it.
    pub result: Option<QscdIndication>,
}

/// Table 6's column for the `CHECK_1_SET-OF_QE` qualifiers: 1 `QCWithSSCD`, 2
/// `QCSSCDStatusAsInCert` or none of the three, 3 `QCNoSSCD`.
#[must_use]
pub const fn table_6(stated: bool, column: usize) -> QscdIndication {
    table(stated, column)
}

/// Table 7's column for the considered set's qualifiers: 1 `QCWithQSCD` or
/// `QCQSCDManagedOnBehalf`, 2 `QCQSCDStatusAsInCert` or none of them, 3 `QCNoQSCD`; the row is
/// the certificate's `QcSSCD` statement.
#[must_use]
pub const fn table_7(sscd: bool, column: usize) -> QscdIndication {
    table(sscd, column)
}

/// Tables 6 and 7 print the same cells: the list's "with" is yes, its "no" is no, and between
/// them the certificate's own statement decides.
const fn table(stated: bool, column: usize) -> QscdIndication {
    match column {
        1 => QscdIndication::Yes,
        2 if stated => QscdIndication::Yes,
        _ => QscdIndication::No,
    }
}

/// Clause 4.5, PRO-4.5.4-01 to -05, over clause 4.4's outputs for the certificate `examined`
/// describes, whose statements are `statements`, at `date_time`.
#[must_use]
pub fn determine(
    qualification: &Determination,
    statements: &Stated,
    examined: &Examined,
    date_time: Instant,
) -> Qscd {
    let mut out = Qscd {
        status: QcStatus::Passed,
        sub_status: Vec::new(),
        result: None,
    };
    // PRO-4.5.4-02.
    if qualification.status == QcStatus::Failed {
        out.status = QcStatus::Failed;
        out.sub_status = vec![QscdSubStatus::QualificationFailed(
            qualification.sub_status.clone(),
        )];
        return out;
    }
    let sscd = matches!(statements, Stated::Read(read) if read.sscd);
    let has = |set: &[Applied], uri: &str| set.iter().any(|applied| applied.uri == uri);
    let for_esig = qualification.results.contains(&Indication::QcForESig);
    // PRO-4.5.4-03.
    if date_time < super::REGULATION_APPLIES {
        if !for_esig {
            // b) to d).
            out.result = Some(QscdIndication::Indeterminate);
            return out;
        }
        let set = qualification.sets_of_qe[0].as_slice();
        let (with, no, as_in_cert) = (
            has(set, QC_WITH_SSCD),
            has(set, QC_NO_SSCD),
            has(set, QC_SSCD_STATUS_AS_IN_CERT),
        );
        // a) 2).
        if (with && no) || (as_in_cert && (with || no)) {
            out.result = Some(QscdIndication::Indeterminate);
            out.status = QcStatus::PassedWithWarning;
            out.sub_status.push(QscdSubStatus::InconsistentSscd);
            return out;
        }
        // a) 3) and 4).
        let qcp_plus = matches!(&examined.policies, Field::Read(held)
            if held.iter().any(|oid| oid == super::QCP_PLUS));
        let column = if with {
            1
        } else if no {
            3
        } else {
            2
        };
        out.result = Some(table_6(sscd || qcp_plus, column));
        return out;
    }
    // PRO-4.5.4-04: the set of the purpose the certificate was confirmed for.
    let set = if for_esig {
        qualification.sets_of_qe[0].as_slice()
    } else if qualification.results.contains(&Indication::QcForESeal) {
        qualification.sets_of_qe[1].as_slice()
    } else {
        // PRO-4.5.4-05.
        out.result = Some(QscdIndication::Indeterminate);
        return out;
    };
    // b).
    for applied in set {
        if !KNOWN.contains(&applied.uri.as_str()) {
            if applied.critical {
                out.status = QcStatus::Failed;
                out.sub_status = vec![QscdSubStatus::UnknownCriticalQualifiers];
                return out;
            }
            if !out.sub_status.contains(&QscdSubStatus::UnknownQualifiers) {
                out.sub_status.push(QscdSubStatus::UnknownQualifiers);
            }
        }
    }
    let with_qscd = has(set, QC_WITH_QSCD);
    let no_qscd = has(set, QC_NO_QSCD);
    let as_in_cert = has(set, QC_QSCD_STATUS_AS_IN_CERT);
    let on_behalf = has(set, QC_QSCD_MANAGED_ON_BEHALF);
    // c) 1) to 5).
    if (has(set, QC_WITH_SSCD) && !with_qscd)
        || (has(set, QC_NO_SSCD) && !no_qscd)
        || (with_qscd && no_qscd)
        || (on_behalf && no_qscd)
        || (as_in_cert && (with_qscd || no_qscd || on_behalf))
    {
        out.result = Some(QscdIndication::Indeterminate);
        out.status = QcStatus::PassedWithWarning;
        out.sub_status.push(QscdSubStatus::InconsistentQscd);
        return out;
    }
    // d) to g).
    let column = if with_qscd || on_behalf {
        1
    } else if no_qscd {
        3
    } else {
        2
    };
    out.result = Some(table_7(sscd, column));
    out
}
