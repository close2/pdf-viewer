//! ETSI EN 319 412-5's QC statements, read off a signer's certificate.
//!
//! RFC 3739 section 3.2.6 gives a certificate an extension of "statements defining explicit
//! properties of the certificate", and EN 319 412-5 V2.4.1 clause 4.2 defines the four by which an
//! issuer claims compliance with a legislation: `QcCompliance` (clause 4.2.1), `QcSSCD` (4.2.2),
//! `QcType` (4.2.3) and `QcCClegislation` (4.2.4). This module reads those four and names every
//! other statement by its identifier.
//!
//! **A statement is the issuer's claim, and reading it decides no level.** ETSI TS 119 615 clause
//! 4.4 weighs these statements against the qualifications a trusted list states for the issuing
//! service at the best signature time, clause 4.5 decides the creation device, and TS 119 172-4
//! REQ-4.4.2-06 joins the two; until those run, a report names the statements and says nothing
//! qualified (ADR 2835). What is read is therefore what the certificate says, in the clause's own
//! distinctions: EN 319 412-5 Table 1 separates an absent `QcType` from a present one, and
//! EN 319 412-5 Table 1A an absent `QcCClegislation` from a present one, so absence is kept as
//! absence rather than as an empty list.
//!
//! ISO 32000-2 §12.8.3.4.5 is where this is reached from: its validation's trust anchors may be a
//! trusted list's services (ADR 2834), and a listing carries these statements as the start of the
//! level determination that list makes possible (ADR 2853).
//!
//! A certificate is a stranger's bytes: the walk is bounded by [`MAX_ENTRIES`], every refusal is a
//! named [`QcStatementError`], and `fuzz/`'s `qc_statement` target holds the reader to both.

use pdf_pki::der::{OBJECT_IDENTIFIER, Reader, SEQUENCE, Value};
use pdf_pki::x509::dotted;

// Every statement below sits under EN 319 412-5 Annex B's `id-etsi-qcs`, `0.4.0.1862.1` —
// "{ itu-t(0) identified-organization(4) etsi(0) id-qc-profile(1862) 1 }" — whose encoding is the
// five octets each constant starts with; the tests pin each one's dotted form.

/// Clause 4.2.1's `id-etsi-qcs-QcCompliance`, "{ id-etsi-qcs 1 }".
pub const QC_COMPLIANCE: &[u8] = &[0x04, 0x00, 0x8E, 0x46, 0x01, 0x01];
/// Clause 4.2.2's `id-etsi-qcs-QcSSCD`, "{ id-etsi-qcs 4 }".
pub const QC_SSCD: &[u8] = &[0x04, 0x00, 0x8E, 0x46, 0x01, 0x04];
/// Clause 4.2.3's `id-etsi-qcs-QcType`, "{ id-etsi-qcs 6 }".
pub const QC_TYPE: &[u8] = &[0x04, 0x00, 0x8E, 0x46, 0x01, 0x06];
/// Clause 4.2.4's `id-etsi-qcs-QcCClegislation`, "{ id-etsi-qcs 7 }".
pub const QC_CC_LEGISLATION: &[u8] = &[0x04, 0x00, 0x8E, 0x46, 0x01, 0x07];

/// The most statements, types and countries one extension is read for, together.
///
/// A certificate EN 319 412-5 Table 2 describes carries at most seven statements, each
/// with a few members; this is far above that and far below what a hostile extension inside a
/// certificate's own size limit could otherwise make this reader allocate one entry at a time.
pub const MAX_ENTRIES: usize = 256;

/// The purpose clause 4.2.3's `QcType` declares.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum QcType {
    /// `id-etsi-qct-esign`, "{ id-etsi-qcs-QcType 1 }": a certificate for electronic signatures.
    Esign,
    /// `id-etsi-qct-eseal`, "{ id-etsi-qcs-QcType 2 }": a certificate for electronic seals.
    Eseal,
    /// `id-etsi-qct-web`, "{ id-etsi-qcs-QcType 3 }": a certificate for website authentication.
    Web,
    /// Another identifier, which the syntax's extension marker (`id-etsi-qct-web, ...`) admits,
    /// carried as dotted decimal.
    Other(String),
}

/// What a certificate's `qcStatements` extension states, in EN 319 412-5 clause 4.2's terms.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QcStatements {
    /// Clause 4.2.1's `esi4-qcStatement-1`: the issuer claims the certificate is qualified, under
    /// the EU texts or under the legislation [`Self::legislation`] names (EN 319 412-5 Table 1 and
    /// EN 319 412-5 Table 1A).
    pub compliance: bool,
    /// Clause 4.2.2's `esi4-qcStatement-4`: the issuer claims the private key resides in a
    /// qualified signature or seal creation device.
    pub sscd: bool,
    /// Clause 4.2.3's `esi4-qcStatement-6`, where stated: the purposes, in the certificate's
    /// order.
    ///
    /// The clause declares a certificate "issued as one and only one of the purposes", so more
    /// than one here is the issuer departing from it; the list is carried as written, because the
    /// determination that weighs it is TS 119 615 clause 4.4's and not this reader's.
    pub types: Option<Vec<QcType>>,
    /// Clause 4.2.4's `esi4-qcStatement-7`, where stated: ISO 3166-1 alpha-2 country codes.
    ///
    /// QCS-4.2.4-01: a certificate issued under the EU texts carries none, so `Some` says the
    /// claim of [`Self::compliance`] is made under these countries' legislation instead
    /// (EN 319 412-5 Table 1A).
    pub legislation: Option<Vec<String>>,
    /// Every other statement's identifier, as dotted decimal — clause 4.3's generic statements and
    /// any other framework's.
    pub others: Vec<String>,
}

/// What stopped a `qcStatements` extension from being read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum QcStatementError {
    /// The extension is not RFC 3739's `QCStatements ::= SEQUENCE OF QCStatement`, or one
    /// statement is not `SEQUENCE { statementId, statementInfo OPTIONAL }`.
    #[error("the qcStatements extension is not a sequence of statements (RFC 3739 section 3.2.6)")]
    Malformed,
    /// A statement EN 319 412-5 defines does not have the syntax its clause gives it.
    #[error(
        "the {statement} statement does not have the syntax ETSI EN 319 412-5 clause {clause} gives it"
    )]
    Syntax {
        /// The statement's ASN.1 name.
        statement: &'static str,
        /// The clause that defines it.
        clause: &'static str,
    },
    /// One of clause 4.2's statements appears twice, so the certificate makes its claim twice
    /// and the two may differ.
    #[error("the {0} statement appears more than once")]
    Repeated(&'static str),
    /// More than [`MAX_ENTRIES`] statements and members.
    #[error("the qcStatements extension holds more than {MAX_ENTRIES} entries")]
    TooMany,
}

impl QcStatements {
    /// Reads RFC 3739's `QCStatements` from the extension's `extnValue` octets
    /// (`pdf_pki::x509::Extensions::qc_statements`).
    ///
    /// # Errors
    ///
    /// [`QcStatementError`], naming what was not read; nothing is defaulted on the issuer's behalf.
    pub fn read(octets: &[u8]) -> Result<Self, QcStatementError> {
        let mut budget = MAX_ENTRIES;
        let mut spend = || {
            budget = budget.checked_sub(1).ok_or(QcStatementError::TooMany)?;
            Ok::<(), QcStatementError>(())
        };
        let mut outer = Reader::new(octets).map_err(|_| QcStatementError::Malformed)?;
        let list = outer
            .next_value()
            .map_err(|_| QcStatementError::Malformed)?
            .filter(|value| value.identifier == SEQUENCE)
            .ok_or(QcStatementError::Malformed)?;
        if !matches!(outer.next_value(), Ok(None)) {
            return Err(QcStatementError::Malformed);
        }
        let mut read = Self::default();
        let mut statements = list.children().map_err(|_| QcStatementError::Malformed)?;
        while let Some(statement) = next(&mut statements)? {
            spend()?;
            if statement.identifier != SEQUENCE {
                return Err(QcStatementError::Malformed);
            }
            let mut members = statement
                .children()
                .map_err(|_| QcStatementError::Malformed)?;
            let id = next(&mut members)?
                .and_then(|value| value.object_identifier())
                .ok_or(QcStatementError::Malformed)?;
            let info = next(&mut members)?;
            match id {
                QC_COMPLIANCE => {
                    if std::mem::replace(&mut read.compliance, true) {
                        return Err(QcStatementError::Repeated("QcCompliance"));
                    }
                }
                QC_SSCD => {
                    if std::mem::replace(&mut read.sscd, true) {
                        return Err(QcStatementError::Repeated("QcSSCD"));
                    }
                }
                QC_TYPE => {
                    if read.types.is_some() {
                        return Err(QcStatementError::Repeated("QcType"));
                    }
                    read.types = Some(types(info, &mut spend)?);
                }
                QC_CC_LEGISLATION => {
                    if read.legislation.is_some() {
                        return Err(QcStatementError::Repeated("QcCClegislation"));
                    }
                    read.legislation = Some(countries(info, &mut spend)?);
                }
                other => read.others.push(name(other)),
            }
        }
        Ok(read)
    }
}

/// The next value of a reader, a malformed encoding refused as [`QcStatementError::Malformed`].
fn next<'a>(reader: &mut Reader<'a>) -> Result<Option<Value<'a>>, QcStatementError> {
    reader.next_value().map_err(|_| QcStatementError::Malformed)
}

/// An identifier as dotted decimal, the way every refusal in this crate names one.
fn name(oid: &[u8]) -> String {
    dotted(oid).unwrap_or_else(|| "an unreadable object identifier".to_owned())
}

/// Clause 4.2.3's `QcType ::= SEQUENCE OF OBJECT IDENTIFIER`.
fn types(
    info: Option<Value<'_>>,
    spend: &mut impl FnMut() -> Result<(), QcStatementError>,
) -> Result<Vec<QcType>, QcStatementError> {
    let syntax = QcStatementError::Syntax {
        statement: "QcType",
        clause: "4.2.3",
    };
    let info = info
        .filter(|value| value.identifier == SEQUENCE)
        .ok_or_else(|| syntax.clone())?;
    let mut members = info.children().map_err(|_| syntax.clone())?;
    let mut types = Vec::new();
    while let Some(member) = members.next_value().map_err(|_| syntax.clone())? {
        spend()?;
        if member.identifier != OBJECT_IDENTIFIER {
            return Err(syntax);
        }
        let oid = member.object_identifier().ok_or_else(|| syntax.clone())?;
        types.push(match oid.strip_prefix(QC_TYPE) {
            Some([1]) => QcType::Esign,
            Some([2]) => QcType::Eseal,
            Some([3]) => QcType::Web,
            _ => QcType::Other(name(oid)),
        });
    }
    Ok(types)
}

/// Clause 4.2.4's `QcCClegislation ::= SEQUENCE OF CountryName`, each a `PrintableString (SIZE
/// (2))` "CONSTRAINED BY ... ISO 3166-1 alpha-2 codes only".
///
/// The size and the alphabet are checked — two letters — and the registry is not: whether a pair
/// of letters is an assigned ISO 3166-1 code is a fact about a list this program does not carry.
fn countries(
    info: Option<Value<'_>>,
    spend: &mut impl FnMut() -> Result<(), QcStatementError>,
) -> Result<Vec<String>, QcStatementError> {
    /// X.680's `PrintableString` universal tag 19.
    const PRINTABLE_STRING: u8 = 0x13;
    let syntax = QcStatementError::Syntax {
        statement: "QcCClegislation",
        clause: "4.2.4",
    };
    let info = info
        .filter(|value| value.identifier == SEQUENCE)
        .ok_or_else(|| syntax.clone())?;
    let mut members = info.children().map_err(|_| syntax.clone())?;
    let mut countries = Vec::new();
    while let Some(member) = members.next_value().map_err(|_| syntax.clone())? {
        spend()?;
        match member.contents {
            &[first, second]
                if member.identifier == PRINTABLE_STRING
                    && first.is_ascii_alphabetic()
                    && second.is_ascii_alphabetic() =>
            {
                countries.push(String::from_utf8_lossy(member.contents).into_owned());
            }
            _ => return Err(syntax),
        }
    }
    Ok(countries)
}

/// What the signer's certificate states, for a verdict to carry.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Stated {
    /// The certificate has no `qcStatements` extension.
    Nothing,
    /// The statements, read.
    Read(QcStatements),
    /// The extension is there and could not be read.
    Unreadable(QcStatementError),
}

impl Stated {
    /// What `certificate` states.
    #[must_use]
    pub fn of(certificate: &pdf_pki::x509::Certificate<'_>) -> Self {
        match certificate.extensions.qc_statements {
            None => Self::Nothing,
            Some(octets) => match QcStatements::read(octets) {
                Ok(read) => Self::Read(read),
                Err(error) => Self::Unreadable(error),
            },
        }
    }
}

#[cfg(test)]
mod tests;
