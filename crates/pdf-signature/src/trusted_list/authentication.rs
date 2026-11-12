//! ETSI TS 119 615 clause 4.1 (authenticating the European Commission's list of trusted lists)
//! and clause 4.2 (authenticating a Member State's list), over the files a host supplied.
//!
//! Both procedures validate a list's enveloped XML signature "considering" one certificate "as a
//! trust anchor" ([`crate::xml_signature`]) and test which certificates the certificate belongs
//! to: the list of trusted lists (LOTL) against the certificates the Official Journal published
//! (GPR-4.0-02's `OJEU-LOTL-Certs-Set`), through each pivot LOTL the LOTL's scheme information
//! URIs name (PRO-4.1.4-11); a Member State's list against the certificates the authenticated
//! LOTL's pointer for its territory names (PRO-4.2.4-05). The statuses and sub-statuses are the
//! clauses' own indications, so a report can quote them.
//!
//! # A file a host supplied is a file the procedure obtained
//!
//! The procedures name each file by the location it is obtained from, and this crate opens no
//! socket (`CLAUDE.md` principle 3): a host keeps the files and hands over their octets with
//! their names. A file the procedure obtains from a URI is the supplied file whose name is the
//! URI's last path segment — what a copy saved under its own name is called — and a Member
//! State's list is the supplied list whose scheme territory is the country code clause 4.2 is run
//! for, because clause 4.2's input is the country code and the location is only where it is
//! fetched from. The configuration GPR-4.0-02 requires is supplied the same way, as files named
//! after its parameters ([`OJEU_LOC`], [`OJEU_LOTL_LOC`], and [`OJEU_LOTL_CERTS_SET`] as the
//! prefix of each certificate's file); the first two default to the values NOTE 1 of GPR-4.0-02
//! records. ADR 2842.
//!
//! # When the procedures run
//!
//! Where a host supplied neither a LOTL nor a certificate of the Official Journal's, there is
//! nothing to authenticate against, and every list stays
//! [`super::Authentication::VouchedForByTheHost`], said as such. Where it supplied either, the
//! procedures run, and a list they do not authenticate is refused by name and not searched.
//!
//! # What is not done
//!
//! PRO-4.1.4-16 and -17 update the configuration from an authenticated LOTL; this crate keeps no
//! state between readings, so the update is a host's, and the procedure's output is what was
//! computed from the files given.

use super::{Authentication, Lists, MAX_ENTRIES, MAX_LIST_OCTETS, TrustedList, XML_MIME_TYPE};
use crate::x509::Instant;
use crate::xml::base64;
use crate::xml_signature::{self, XmlSignatureError};

/// GPR-4.0-02's `OJEU-Loc`, as the name of the file a host may supply it in.
pub const OJEU_LOC: &str = "OJEU-Loc";

/// GPR-4.0-02's `OJEU-LOTL-Loc`, as the name of the file a host may supply it in.
pub const OJEU_LOTL_LOC: &str = "OJEU-LOTL-Loc";

/// GPR-4.0-02's `OJEU-LOTL-Certs-Set`, as the prefix of the names of the files a host supplies
/// its certificates in: each DER, or PEM `CERTIFICATE` blocks.
pub const OJEU_LOTL_CERTS_SET: &str = "OJEU-LOTL-Certs-Set";

/// The value GPR-4.0-02 NOTE 1 records for `OJEU-Loc` at the specification's publication.
pub const OJEU_LOC_AT_PUBLICATION: &str =
    "https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=uriserv:OJ.C_.2019.276.01.0001.01.ENG";

/// The value GPR-4.0-02 NOTE 1 records for `OJEU-LOTL-Loc` at the specification's publication.
pub const OJEU_LOTL_LOC_AT_PUBLICATION: &str = "https://ec.europa.eu/tools/lotl/eu-lotl.xml";

/// Clause 4.1.3's outputs, or clause 4.2.3's, as status and sub-status indications.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// `LOTL-Status` or `EUTL-Status`.
    pub status: &'static str,
    /// `LOTL-Sub-Status` or `EUTL-Sub-Status`: the clause's indications, each followed where the
    /// signature validation said why by the reason, after a colon.
    pub sub_status: Vec<String>,
}

impl Outcome {
    /// Whether the status is clause 4.1.3's or 4.2.3's passing indication.
    #[must_use]
    pub fn passed(&self) -> bool {
        matches!(
            self.status,
            "LOTL_VERIFICATION_PASSED" | "EUTL_VERIFICATION_PASSED"
        )
    }

    fn failed(status: &'static str, indication: impl Into<String>) -> Self {
        Self {
            status,
            sub_status: vec![indication.into()],
        }
    }
}

impl std::fmt::Display for Outcome {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.status)?;
        if !self.sub_status.is_empty() {
            write!(formatter, " ({})", self.sub_status.join("; "))?;
        }
        Ok(())
    }
}

/// Clause 4.1's `Authenticated-LOTL`, as far as clause 4.2 reads it.
#[derive(Debug)]
pub struct AuthenticatedLotl {
    /// The LOTL.
    pub list: TrustedList,
    /// The outcome, `LOTL_VERIFICATION_PASSED`.
    pub outcome: Outcome,
}

/// GPR-4.0-02's three parameters, as the host supplied them.
pub(super) struct Configuration {
    pub(super) ojeu_loc: String,
    pub(super) ojeu_lotl_loc: String,
    pub(super) certificates: Vec<Vec<u8>>,
    /// Whether the host supplied any of the three, which is what makes it ask for the procedures.
    pub(super) supplied: bool,
}

impl Configuration {
    pub(super) fn of(lists: &Lists) -> Self {
        let text = |name: &str| {
            lists
                .lists()
                .iter()
                .find(|(file, _)| file == name)
                .and_then(|(_, octets)| std::str::from_utf8(octets).ok())
                .map(|value| value.trim().to_owned())
        };
        let ojeu_loc = text(OJEU_LOC);
        let ojeu_lotl_loc = text(OJEU_LOTL_LOC);
        let mut certificates = Vec::new();
        for (name, octets) in lists.lists() {
            if name.starts_with(OJEU_LOTL_CERTS_SET) {
                certificates.extend(certificates_in(octets));
            }
        }
        Self {
            supplied: ojeu_loc.is_some() || ojeu_lotl_loc.is_some() || !certificates.is_empty(),
            ojeu_loc: ojeu_loc.unwrap_or_else(|| OJEU_LOC_AT_PUBLICATION.to_owned()),
            ojeu_lotl_loc: ojeu_lotl_loc.unwrap_or_else(|| OJEU_LOTL_LOC_AT_PUBLICATION.to_owned()),
            certificates,
        }
    }

    /// Whether a supplied file is one of the parameters rather than a list.
    pub(super) fn names(name: &str) -> bool {
        name == OJEU_LOC || name == OJEU_LOTL_LOC || name.starts_with(OJEU_LOTL_CERTS_SET)
    }
}

/// A DER certificate where the file begins with X.690's `SEQUENCE` tag, and otherwise every PEM
/// `CERTIFICATE` block (RFC 7468 section 5) it holds.
fn certificates_in(octets: &[u8]) -> Vec<Vec<u8>> {
    const BEGIN: &str = "-----BEGIN CERTIFICATE-----";
    const END: &str = "-----END CERTIFICATE-----";
    if octets.first() == Some(&0x30) {
        return vec![octets.to_vec()];
    }
    let Ok(text) = std::str::from_utf8(octets) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(BEGIN) {
        let after = rest
            .get(start.saturating_add(BEGIN.len())..)
            .unwrap_or_default();
        let Some(end) = after.find(END) else {
            break;
        };
        if let Some(der) = after.get(..end).and_then(base64) {
            found.push(der);
        }
        rest = after
            .get(end.saturating_add(END.len())..)
            .unwrap_or_default();
    }
    found
}

/// The last path segment of a URI: the name a copy saved from it carries.
pub(super) fn file_name(uri: &str) -> &str {
    let path = uri.split(['?', '#']).next().unwrap_or(uri);
    path.rsplit('/').next().unwrap_or(path)
}

/// RFC 3986's shape PRO-4.1.4-04 counts: the `https` scheme, an authority, and a non-empty path,
/// ending `.xml` where `xml` holds.
fn is_https_with_path(uri: &str, xml: bool) -> bool {
    let Some(rest) = uri.strip_prefix("https://") else {
        return false;
    };
    let Some((authority, path)) = rest.split_once('/') else {
        return false;
    };
    let path = path.split(['?', '#']).next().unwrap_or(path);
    // The clause's string ".xml", as written: a comparison of octets, not of an extension.
    !authority.is_empty() && !path.is_empty() && path.as_bytes().ends_with(b".xml") == xml
}

/// The certificates of the LOTL's pointer to itself, PRO-4.1.4-09 b): every tuple whose scheme
/// territory qualifier is `EU`.
fn eu_pointer_certificates(list: &TrustedList) -> Vec<Vec<u8>> {
    list.pointers
        .iter()
        .filter(|pointer| pointer.territory.as_deref() == Some("EU"))
        .flat_map(|pointer| pointer.certificates.iter().cloned())
        .collect()
}

/// The supplied octets of the file obtained from `uri`.
fn obtained<'l>(lists: &'l Lists, uri: &str) -> Option<&'l [u8]> {
    let name = file_name(uri);
    lists
        .lists()
        .iter()
        .find(|(file, _)| file == name)
        .map(|(_, octets)| octets.as_slice())
}

/// The validation PRO-4.1.4-07, PRO-4.1.4-11 e) and PRO-4.2.4-08 run: the list's signature under
/// the certificate its `ds:KeyInfo` carries, which is then that list's signer.
fn validated(octets: &[u8]) -> Result<Vec<u8>, XmlSignatureError> {
    if octets.len() > MAX_LIST_OCTETS {
        return Err(XmlSignatureError::Unreadable(format!(
            "{} octets, past the {MAX_LIST_OCTETS} this program reads",
            octets.len()
        )));
    }
    xml_signature::verify_enveloped(octets, MAX_ENTRIES).map(|verified| verified.signer)
}

const LOTL_FAILED: &str = "LOTL_VERIFICATION_FAILED";
const EUTL_FAILED: &str = "EUTL_VERIFICATION_FAILED";

/// TS 119 615 clause 4.1 over the supplied files, at `at` (PRO-4.1.4-13's "has passed").
///
/// # Errors
///
/// The [`Outcome`] with `LOTL_VERIFICATION_FAILED` and the sub-status that stopped it. A file the
/// procedure obtains and the host did not supply is named in a sub-status of this program's own,
/// `<location> NOT_SUPPLIED`, because the clause states none for a file it could not obtain.
pub(super) fn lotl(
    lists: &Lists,
    configuration: &Configuration,
    at: Instant,
) -> Result<AuthenticatedLotl, Outcome> {
    let not_supplied = |uri: &str| Outcome::failed(LOTL_FAILED, format!("{uri} NOT_SUPPLIED"));
    // PRO-4.1.4-01.
    let octets = obtained(lists, &configuration.ojeu_lotl_loc)
        .ok_or_else(|| not_supplied(&configuration.ojeu_lotl_loc))?;
    let lotl = TrustedList::read(octets)
        .map_err(|error| Outcome::failed(LOTL_FAILED, format!("LOTL_UNREADABLE: {error}")))?;
    let mut warnings = Vec::new();
    // PRO-4.1.4-03.
    let Some(position) = lotl
        .scheme_information_uris
        .iter()
        .position(|uri| *uri == configuration.ojeu_loc)
    else {
        return Err(Outcome::failed(
            LOTL_FAILED,
            "OJEU_LOCATION_INPUT_NOT_MATCHING_OJEU_LOCATION_IN_LOTL",
        ));
    };
    // PRO-4.1.4-04: the pivots, newest first, up to the Official Journal's URI.
    let pivots: Vec<&String> = lotl
        .scheme_information_uris
        .iter()
        .take(position)
        .filter(|uri| is_https_with_path(uri, true))
        .collect();
    // PRO-4.1.4-05 and -06: the LOTL's pointer to itself. A host that supplied the file under the
    // same name supplied this LOTL; one under another name is compared octet for octet; one not
    // supplied could not be obtained, which GPR-4.0-03 makes a warning rather than a stop.
    if let Some(location) = lotl
        .pointers
        .iter()
        .find(|pointer| pointer.territory.as_deref() == Some("EU"))
        .map(|pointer| pointer.location.as_str())
    {
        match obtained(lists, location) {
            Some(other) if other != octets && location != configuration.ojeu_lotl_loc => {
                return Err(Outcome::failed(LOTL_FAILED, "LOTL_FILE_CONFLICT"));
            }
            Some(_) => {}
            None => warnings.push(format!("WARNING_{location}_NOT_SUPPLIED")),
        }
    }
    // PRO-4.1.4-02, -07 and -08.
    let mut lotlso_cert = validated(octets).map_err(|error| {
        Outcome::failed(
            LOTL_FAILED,
            format!("LOTL_SIGNATURE_VERIFICATION_FAILED: {error}"),
        )
    })?;
    // PRO-4.1.4-09 b), -10.
    let mut certs_set = eu_pointer_certificates(&lotl);
    if pivots.is_empty() && !certs_set.contains(&lotlso_cert) {
        return Err(Outcome::failed(
            LOTL_FAILED,
            "LOTL_SIGNER_CERT_NOT_AUTHENTICATED_BY_LOTL",
        ));
    }
    // PRO-4.1.4-11.
    for (index, uri) in pivots.iter().enumerate() {
        let i = index.saturating_add(1);
        let pivot_octets = obtained(lists, uri).ok_or_else(|| not_supplied(uri))?;
        let pivot = TrustedList::read(pivot_octets).map_err(|error| {
            Outcome::failed(LOTL_FAILED, format!("PIVOT_{i}_UNREADABLE: {error}"))
        })?;
        certs_set = eu_pointer_certificates(&pivot);
        if !certs_set.contains(&lotlso_cert) {
            return Err(Outcome::failed(
                LOTL_FAILED,
                format!("PIVOT_{index}_SIGNER_CERT_NOT_AUTHENTICATED_BY_PIVOT_{i}"),
            ));
        }
        lotlso_cert = validated(pivot_octets).map_err(|error| {
            Outcome::failed(
                LOTL_FAILED,
                format!("PIVOT_{i}_SIGNATURE_VERIFICATION_FAILED: {error}"),
            )
        })?;
        if !certs_set.contains(&lotlso_cert) {
            return Err(Outcome::failed(
                LOTL_FAILED,
                format!("PIVOT_{i}_SIGNER_CERT_NOT_AUTHENTICATED_BY_PIVOT_{i}"),
            ));
        }
    }
    // PRO-4.1.4-12.
    if !configuration.certificates.contains(&lotlso_cert) {
        return Err(Outcome::failed(
            LOTL_FAILED,
            "PIVOT_n_LOTLSO_SIGNER_CERT_NOT_AUTHENTICATED_BY_OJEU",
        ));
    }
    // PRO-4.1.4-13.
    if lotl
        .next_update
        .is_some_and(|next| next.unix_seconds() < at.unix_seconds())
    {
        return Err(Outcome::failed(LOTL_FAILED, "LOTL_NEXTUPDATE_PASSED"));
    }
    // PRO-4.1.4-14 and -15.
    Ok(AuthenticatedLotl {
        list: lotl,
        outcome: Outcome {
            status: "LOTL_VERIFICATION_PASSED",
            sub_status: warnings,
        },
    })
}

/// TS 119 615 clause 4.2 for the supplied list `list` (its octets `octets`), the country code
/// being its own scheme territory, under `lotl`, at `at`.
///
/// # Errors
///
/// The [`Outcome`] with `EUTL_VERIFICATION_FAILED` and the sub-status that stopped it; where the
/// LOTL names no XML list for the territory, the sub-status is this program's own
/// `NO_POINTER_FOR_<CC>`, because the clause has nothing to obtain and states none.
pub(super) fn member_state(
    list: &mut TrustedList,
    octets: &[u8],
    lotl: &AuthenticatedLotl,
    at: Instant,
) -> Result<Outcome, Outcome> {
    let territory = list.territory.clone();
    // PRO-4.2.4-03 and -05.
    let pointers: Vec<_> = lotl
        .list
        .pointers
        .iter()
        .filter(|pointer| {
            pointer.territory.as_deref() == Some(territory.as_str())
                && pointer.mime_type.as_deref() == Some(XML_MIME_TYPE)
        })
        .collect();
    if pointers.is_empty() {
        return Err(Outcome::failed(
            EUTL_FAILED,
            format!("NO_POINTER_FOR_{territory}"),
        ));
    }
    let certs_set: Vec<&Vec<u8>> = pointers
        .iter()
        .flat_map(|pointer| &pointer.certificates)
        .collect();
    // PRO-4.2.4-06 and -07.
    let signer = xml_signature::signer_of(octets, MAX_ENTRIES).map_err(|error| {
        Outcome::failed(
            EUTL_FAILED,
            format!("EUTL_SIGNATURE_VERIFICATION_FAILED: {error}"),
        )
    })?;
    if !certs_set.contains(&&signer) {
        return Err(Outcome::failed(
            EUTL_FAILED,
            "EUTLSO_SIGNER_CERT_NOT_AUTHENTICATED_BY_LOTL",
        ));
    }
    // PRO-4.2.4-08 and -09.
    validated(octets).map_err(|error| {
        Outcome::failed(
            EUTL_FAILED,
            format!("EUTL_SIGNATURE_VERIFICATION_FAILED: {error}"),
        )
    })?;
    // PRO-4.2.4-10.
    let mut sub_status = Vec::new();
    if list
        .next_update
        .is_some_and(|next| next.unix_seconds() < at.unix_seconds())
    {
        sub_status.push("WARNING_EUTL_NEXTUPDATE_PASSED".to_owned());
    }
    // PRO-4.2.4-11 and -12.
    list.authentication = Authentication::Authenticated {
        lotl_sequence_number: lotl.list.sequence_number,
    };
    Ok(Outcome {
        status: "EUTL_VERIFICATION_PASSED",
        sub_status,
    })
}

#[cfg(test)]
pub(crate) mod tests;
