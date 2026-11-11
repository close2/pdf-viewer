//! RFC 5755's attribute certificate, read for what section 5's validation asks of it.
//!
//! An attribute certificate binds attributes to a holder under an attribute authority's
//! signature. ETSI EN 319 122-1 clause 5.2.6.1 and RFC 5126 section 5.11.3 let a signer carry
//! one as a certified signer attribute, and a signature policy's attribute trust condition says
//! under which trust trees one is believed (`crate::policy_rules::signer_attributes`). This
//! module reads one; it judges nothing.
//!
//! # Tagging
//!
//! RFC 5755 Appendix B's module is `DEFINITIONS IMPLICIT TAGS`, so a context tag *replaces* the
//! tag of the type it names: `Holder`'s `baseCertificateID [0] IssuerSerial` is an `[0]` holding
//! `IssuerSerial`'s members, `entityName [1] GeneralNames` an `[1]` holding the names, and
//! `AttCertIssuer`'s `v2Form [0] V2Form` an `[0]` holding `V2Form`'s members. `GeneralName`'s
//! `directoryName [4] Name` is the exception every `IMPLICIT` module shares: `Name` is a `CHOICE`,
//! and X.680 makes a tag on a `CHOICE` explicit, so `[4]` wraps the `Name`'s `SEQUENCE`.
//!
//! # What is refused as the profile's violation
//!
//! Section 4.2.1 makes the version v2, section 4.2.3 makes the issuer the `v2Form` holding one
//! `GeneralName`, a non-empty `directoryName`, and section 4.2.7 makes the attribute list
//! non-empty with each type once. Section 4.2.6 writes both instants as `GeneralizedTime` in UTC
//! with seconds and no fraction. A certificate that breaks one of these is [`Unreadable::Malformed`]:
//! it is not the profile's attribute certificate, so nothing here can call it valid.

use crate::der::{OBJECT_IDENTIFIER, OCTET_STRING, SEQUENCE, SET, Value};
use crate::x509::{self, Validity};

/// At most this many attributes are read out of one attribute certificate.
///
/// Section 4.2.7 bounds the list below at one and states no upper bound; an attribute
/// certificate carries the handful of roles, groups or clearances its holder is granted, and a
/// list past this is refused rather than read without end.
pub const MAX_ATTRIBUTES: usize = 64;

/// At most this many extensions, and this many holder names, are read.
///
/// Section 4.3 defines six extensions and section 4.2.2 recommends one holder name; a list past
/// this is refused rather than read without end.
pub const MAX_ENTRIES: usize = 32;

/// `id-pe-ac-auditIdentity`, section 4.3.1: 1.3.6.1.5.5.7.1.4.
const AUDIT_IDENTITY: &[u8] = &[0x2B, 0x06, 0x01, 0x05, 0x05, 0x07, 0x01, 0x04];
/// `id-ce-targetInformation`, section 4.3.2: 2.5.29.55.
const TARGET_INFORMATION: &[u8] = &[0x55, 0x1D, 0x37];
/// `id-ce-authorityKeyIdentifier`, section 4.3.3: 2.5.29.35.
const AUTHORITY_KEY_IDENTIFIER: &[u8] = &[0x55, 0x1D, 0x23];
/// `id-pe-authorityInfoAccess`, section 4.3.4: 1.3.6.1.5.5.7.1.1.
const AUTHORITY_INFO_ACCESS: &[u8] = &[0x2B, 0x06, 0x01, 0x05, 0x05, 0x07, 0x01, 0x01];
/// `id-ce-cRLDistributionPoints`, section 4.3.5: 2.5.29.31.
const CRL_DISTRIBUTION_POINTS: &[u8] = &[0x55, 0x1D, 0x1F];
/// `id-ce-noRevAvail`, section 4.3.6: 2.5.29.56.
const NO_REVOCATION_AVAILABLE: &[u8] = &[0x55, 0x1D, 0x38];

/// The extensions this reader supports in section 5's sense: their presence never rejects a
/// certificate here, because none of them states a condition on its validity that a signature's
/// verifier must apply. The audit identity names the holder for an audit trail, the authority key
/// identifier, the information access and the distribution points say where to look for the
/// issuer and its revocation information, and `noRevAvail` says there is none. Section 4.3.2's
/// targeting does state a condition, and it is reported as [`AttributeCertificate::targeted`].
const SUPPORTED: [&[u8]; 5] = [
    AUDIT_IDENTITY,
    AUTHORITY_KEY_IDENTIFIER,
    AUTHORITY_INFO_ACCESS,
    CRL_DISTRIBUTION_POINTS,
    NO_REVOCATION_AVAILABLE,
];

/// Section 4.1's `IssuerSerial`, as section 4.2.2 profiles it for a holder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IssuerSerial<'a> {
    /// The contents of the `Name` in the one `directoryName` the issuer's `GeneralNames` holds,
    /// as [`x509::Certificate::issuer`] keeps a certificate's; `None` where the `GeneralNames` is
    /// anything else, which section 4.2.2 does not let a conforming holder write.
    pub issuer: Option<&'a [u8]>,
    /// The serial number's contents.
    pub serial: &'a [u8],
    /// Whether `issuerUID` is present.
    pub issuer_uid: bool,
}

/// Section 4.1's `Holder`: the three ways an attribute certificate names whom it is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holder<'a> {
    /// `baseCertificateID`: the issuer and serial number of the holder's public-key certificate.
    pub base: Option<IssuerSerial<'a>>,
    /// `entityName`: each `GeneralName`, whole.
    pub entity: Vec<Value<'a>>,
    /// Whether `objectDigestInfo` is present, which section 4.2.2 does not require a verifier to
    /// support.
    pub digest: bool,
}

/// One attribute certificate, as section 4.1 defines it and section 4.2 profiles it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeCertificate<'a> {
    /// `acinfo`'s whole encoding: the octets the issuer's signature covers.
    pub info: &'a [u8],
    /// Whether `acinfo` stated X.690's indefinite length, so that its signed extent was found by
    /// a scan rather than read from a length, as [`x509::Certificate::indefinite_lengths`] says
    /// of a `tbsCertificate`.
    pub indefinite: bool,
    /// Whom the certificate is about.
    pub holder: Holder<'a>,
    /// The contents of the issuer's `Name`.
    pub issuer: &'a [u8],
    /// The serial number's contents.
    pub serial: &'a [u8],
    /// `attrCertValidityPeriod`.
    pub validity: Validity,
    /// The type of each attribute, as an object identifier's contents.
    pub types: Vec<&'a [u8]>,
    /// Whether section 4.3.2's target information extension is present.
    pub targeted: bool,
    /// The identifier of the first critical extension this reader does not support, where one is
    /// present.
    pub unsupported_critical: Option<&'a [u8]>,
    /// `signatureAlgorithm`'s object identifier.
    pub signature_algorithm: &'a [u8],
    /// `signatureAlgorithm`'s parameters, where stated.
    pub signature_parameters: Option<Value<'a>>,
    /// `signatureValue`'s octets.
    pub signature: &'a [u8],
}

/// Why an attribute certificate was not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Unreadable {
    /// The named member is not section 4.1's syntax as section 4.2 profiles it.
    #[error("its {0} is not RFC 5755's syntax")]
    Malformed(&'static str),
    /// The named list holds more entries than this program reads.
    #[error("its {0} hold more entries than this program reads")]
    TooMany(&'static str),
}

/// The next member of `reader`, of this identifier.
fn member<'a>(
    reader: &mut crate::der::Reader<'a>,
    identifier: u8,
    name: &'static str,
) -> Result<Value<'a>, Unreadable> {
    match reader.next_value() {
        Ok(Some(value)) if value.identifier == identifier => Ok(value),
        _ => Err(Unreadable::Malformed(name)),
    }
}

/// The children of a constructed value, each one, under [`MAX_ENTRIES`] or `bound`.
fn entries<'a>(
    value: &Value<'a>,
    bound: usize,
    name: &'static str,
) -> Result<Vec<Value<'a>>, Unreadable> {
    let mut reader = value.children().map_err(|_| Unreadable::Malformed(name))?;
    let mut out = Vec::new();
    while let Some(entry) = reader
        .next_value()
        .map_err(|_| Unreadable::Malformed(name))?
    {
        if out.len() == bound {
            return Err(Unreadable::TooMany(name));
        }
        out.push(entry);
    }
    Ok(out)
}

/// The contents of the `Name` where `names`' members are exactly one non-empty `directoryName`.
fn one_directory_name<'a>(names: &[Value<'a>]) -> Option<&'a [u8]> {
    let [name] = names else { return None };
    if name.identifier != 0xA4 {
        return None;
    }
    let mut inner = name.children().ok()?;
    match (inner.next_value(), inner.next_value()) {
        (Ok(Some(rdns)), Ok(None)) if rdns.identifier == SEQUENCE && !rdns.contents.is_empty() => {
            Some(rdns.contents)
        }
        _ => None,
    }
}

/// `Holder`, section 4.2.2.
fn read_holder<'a>(holder: &Value<'a>) -> Result<Holder<'a>, Unreadable> {
    let mut out = Holder {
        base: None,
        entity: Vec::new(),
        digest: false,
    };
    let mut previous = None;
    for option in entries(holder, 3, "holder")? {
        let number = (option.identifier & 0xE0 == 0xA0)
            .then_some(option.identifier & 0x1F)
            .filter(|number| *number <= 2 && previous.is_none_or(|last| *number > last))
            .ok_or(Unreadable::Malformed("holder"))?;
        previous = Some(number);
        match number {
            0 => {
                // `IssuerSerial ::= SEQUENCE { issuer GeneralNames, serial, issuerUID OPTIONAL }`,
                // its own tag replaced by `[0]`.
                let mut members = option
                    .children()
                    .map_err(|_| Unreadable::Malformed("holder"))?;
                let issuer = member(&mut members, SEQUENCE, "holder")?;
                let serial = member(&mut members, crate::der::INTEGER, "holder")?;
                let issuer_uid = members
                    .next_value()
                    .map_err(|_| Unreadable::Malformed("holder"))?
                    .is_some();
                out.base = Some(IssuerSerial {
                    issuer: one_directory_name(&entries(&issuer, MAX_ENTRIES, "holder")?),
                    serial: serial.contents,
                    issuer_uid,
                });
            }
            1 => out.entity = entries(&option, MAX_ENTRIES, "holder")?,
            _ => out.digest = true,
        }
    }
    if out.base.is_none() && out.entity.is_empty() && !out.digest {
        return Err(Unreadable::Malformed("holder"));
    }
    Ok(out)
}

/// `AttCertIssuer`, section 4.2.3: the `v2Form` holding only `issuerName`, one non-empty
/// `directoryName`.
fn read_issuer<'a>(issuer: &Value<'a>) -> Result<&'a [u8], Unreadable> {
    if issuer.identifier != 0xA0 {
        return Err(Unreadable::Malformed("issuer"));
    }
    let mut members = issuer
        .children()
        .map_err(|_| Unreadable::Malformed("issuer"))?;
    let names = member(&mut members, SEQUENCE, "issuer")?;
    if !matches!(members.next_value(), Ok(None)) {
        return Err(Unreadable::Malformed("issuer"));
    }
    one_directory_name(&entries(&names, MAX_ENTRIES, "issuer")?)
        .ok_or(Unreadable::Malformed("issuer"))
}

/// `attributes`, section 4.2.7: each type, at least one, none twice.
fn read_types<'a>(attributes: &Value<'a>) -> Result<Vec<&'a [u8]>, Unreadable> {
    let mut types: Vec<&[u8]> = Vec::new();
    for attribute in entries(attributes, MAX_ATTRIBUTES, "attributes")? {
        let mut members = (attribute.identifier == SEQUENCE)
            .then(|| attribute.children().ok())
            .flatten()
            .ok_or(Unreadable::Malformed("attributes"))?;
        let kind = member(&mut members, OBJECT_IDENTIFIER, "attributes")?.contents;
        let values = member(&mut members, SET, "attributes")?;
        if values.contents.is_empty() || types.contains(&kind) {
            return Err(Unreadable::Malformed("attributes"));
        }
        types.push(kind);
    }
    if types.is_empty() {
        return Err(Unreadable::Malformed("attributes"));
    }
    Ok(types)
}

/// `Extensions`: whether the certificate is targeted, and its first unsupported critical
/// extension.
fn read_extensions<'a>(extensions: &Value<'a>) -> Result<(bool, Option<&'a [u8]>), Unreadable> {
    let mut targeted = false;
    let mut unsupported = None;
    for extension in entries(extensions, MAX_ENTRIES, "extensions")? {
        let mut members = (extension.identifier == SEQUENCE)
            .then(|| extension.children().ok())
            .flatten()
            .ok_or(Unreadable::Malformed("extensions"))?;
        let id = member(&mut members, OBJECT_IDENTIFIER, "extensions")?.contents;
        let next = members
            .next_value()
            .map_err(|_| Unreadable::Malformed("extensions"))?;
        let (critical, value) = match next {
            Some(flag) if flag.identifier == 0x01 && flag.contents.len() == 1 => (
                flag.contents != [0],
                members
                    .next_value()
                    .map_err(|_| Unreadable::Malformed("extensions"))?,
            ),
            other => (false, other),
        };
        if value.is_none_or(|value| value.identifier != OCTET_STRING) {
            return Err(Unreadable::Malformed("extensions"));
        }
        if id == TARGET_INFORMATION {
            targeted = true;
        } else if critical && !SUPPORTED.contains(&id) && unsupported.is_none() {
            unsupported = Some(id);
        }
    }
    Ok((targeted, unsupported))
}

/// Reads one `AttributeCertificate`, already read as a value of an enclosing structure.
///
/// # Errors
///
/// [`Unreadable`], naming the member that is not section 4.1's syntax as section 4.2 profiles
/// it, or the list past this program's bound.
pub fn read(certificate: Value<'_>) -> Result<AttributeCertificate<'_>, Unreadable> {
    if certificate.identifier != SEQUENCE {
        return Err(Unreadable::Malformed("structure"));
    }
    let mut outer = certificate
        .children()
        .map_err(|_| Unreadable::Malformed("structure"))?;
    let info = member(&mut outer, SEQUENCE, "acinfo")?;
    let algorithm = member(&mut outer, SEQUENCE, "signatureAlgorithm")?;
    let signature = member(&mut outer, 0x03, "signatureValue")?;
    let (signature_algorithm, signature_parameters) = {
        let mut parts = algorithm
            .children()
            .map_err(|_| Unreadable::Malformed("signatureAlgorithm"))?;
        let oid = member(&mut parts, OBJECT_IDENTIFIER, "signatureAlgorithm")?.contents;
        let parameters = parts
            .next_value()
            .map_err(|_| Unreadable::Malformed("signatureAlgorithm"))?;
        (oid, parameters)
    };
    let Some((0, signature)) = signature.contents.split_first() else {
        return Err(Unreadable::Malformed("signatureValue"));
    };

    let mut members = info
        .children()
        .map_err(|_| Unreadable::Malformed("acinfo"))?;
    // Section 4.2.1: `AttCertVersion ::= INTEGER { v2(1) }`, and v2 it must be.
    if member(&mut members, crate::der::INTEGER, "version")?.contents != [1] {
        return Err(Unreadable::Malformed("version"));
    }
    let holder = read_holder(&member(&mut members, SEQUENCE, "holder")?)?;
    let issuer = read_issuer(
        &members
            .next_value()
            .ok()
            .flatten()
            .ok_or(Unreadable::Malformed("issuer"))?,
    )?;
    member(&mut members, SEQUENCE, "signature")?;
    let serial = member(&mut members, crate::der::INTEGER, "serialNumber")?.contents;
    let validity = {
        let period = member(&mut members, SEQUENCE, "attrCertValidityPeriod")?;
        let mut instants = period
            .children()
            .map_err(|_| Unreadable::Malformed("attrCertValidityPeriod"))?;
        let mut instant = || {
            member(&mut instants, 0x18, "attrCertValidityPeriod")
                .ok()
                .and_then(|time| x509::read_time(&time))
                .ok_or(Unreadable::Malformed("attrCertValidityPeriod"))
        };
        Validity {
            not_before: instant()?,
            not_after: instant()?,
        }
    };
    let types = read_types(&member(&mut members, SEQUENCE, "attributes")?)?;
    let mut targeted = false;
    let mut unsupported_critical = None;
    while let Some(optional) = members
        .next_value()
        .map_err(|_| Unreadable::Malformed("acinfo"))?
    {
        match optional.identifier {
            // `issuerUniqueID UniqueIdentifier` is a `BIT STRING`, carrying nothing a verifier
            // checks here; section 4.2.8 ties it to the issuer's certificate.
            0x03 => {}
            SEQUENCE => (targeted, unsupported_critical) = read_extensions(&optional)?,
            _ => return Err(Unreadable::Malformed("acinfo")),
        }
    }
    Ok(AttributeCertificate {
        info: info.encoding(),
        indefinite: certificate.had_indefinite_length() || info.had_indefinite_length(),
        holder,
        issuer,
        serial,
        validity,
        types,
        targeted,
        unsupported_critical,
        signature_algorithm,
        signature_parameters,
        signature,
    })
}

#[cfg(test)]
pub(crate) mod tests;
