//! TS 119 615 clauses 4.1 and 4.2 over files written and signed here, one indication at a time.
//!
//! The lists are signed by `crate::xml_signature::tests`'s two keys in TS 119 612 Annex B.1.0's
//! shape: the list of trusted lists (territory `EU`) by [`LOTL_KEY`], the Member State's list
//! (territory `AT`) by [`MEMBER_KEY`]. Each test changes one input of the arrangement clause 4.1
//! passes on and names the indication the clause gives for it.

use super::super::tests::{GRANTED_AT_LATER, Written, list_xml};
use super::super::{ADDITIONAL_TYPES, TrustedListError};
use super::*;
use crate::trusted_list::tests::base64_of;
use crate::xml_signature::tests::{Key, LOTL_KEY, MEMBER_KEY, signed};

/// 2026-11-12T00:00:00Z, before the fixtures' next update.
const NOW: i64 = 1_794_441_600;
/// 2027-01-01T00:00:00Z, after it.
const LATE: i64 = 1_798_761_600;

fn pointer(territory: &str, location: &str, keys: &[Key]) -> String {
    let identities = keys
        .iter()
        .map(|key| {
            format!(
                "<tsl:ServiceDigitalIdentity><tsl:DigitalId><tsl:X509Certificate>{}\
                 </tsl:X509Certificate></tsl:DigitalId></tsl:ServiceDigitalIdentity>",
                base64_of(&key.certificate())
            )
        })
        .collect::<Vec<_>>()
        .concat();
    format!(
        "<tsl:OtherTSLPointer><tsl:ServiceDigitalIdentities>{identities}\
         </tsl:ServiceDigitalIdentities><tsl:TSLLocation>{location}</tsl:TSLLocation>\
         <tsl:AdditionalInformation><tsl:OtherInformation><tsl:SchemeTerritory>{territory}\
         </tsl:SchemeTerritory></tsl:OtherInformation><tsl:OtherInformation>\
         <tslx:MimeType xmlns:tslx=\"{ADDITIONAL_TYPES}\">{XML_MIME_TYPE}</tslx:MimeType>\
         </tsl:OtherInformation></tsl:AdditionalInformation></tsl:OtherTSLPointer>"
    )
}

/// Clause 5.3's scheme fields, canonically written, with `uris` as clause 5.3.7's and `pointers`
/// as clause 5.3.13's, and `rest` after the scheme information.
fn scheme(territory: &str, uris: &[&str], pointers: &str, rest: &str) -> String {
    let uris = uris
        .iter()
        .map(|uri| {
            format!(
                "<tsl:URI xml:lang=\"en\">{}</tsl:URI>",
                uri.replace('&', "&amp;")
            )
        })
        .collect::<Vec<_>>()
        .concat();
    format!(
        "<tsl:SchemeInformation><tsl:TSLSequenceNumber>7</tsl:TSLSequenceNumber>\
         <tsl:SchemeInformationURI>{uris}</tsl:SchemeInformationURI>\
         <tsl:SchemeTerritory>{territory}</tsl:SchemeTerritory>\
         <tsl:PointersToOtherTSL>{pointers}</tsl:PointersToOtherTSL>\
         <tsl:ListIssueDateTime>2026-11-01T00:00:00Z</tsl:ListIssueDateTime>\
         <tsl:NextUpdate><tsl:dateTime>2026-12-01T00:00:00Z</tsl:dateTime></tsl:NextUpdate>\
         </tsl:SchemeInformation>{rest}"
    )
}

/// The list of trusted lists, its scheme information URIs `uris`, its `EU` pointer naming
/// `eu_keys` and its `AT` pointer naming [`MEMBER_KEY`], signed by `signer`.
fn lotl(uris: &[&str], eu_keys: &[Key], signer: Key) -> String {
    let pointers = pointer("EU", OJEU_LOTL_LOC_AT_PUBLICATION, eu_keys)
        + &pointer("AT", "https://example.at/tl.xml", &[MEMBER_KEY]);
    signed(&scheme("EU", uris, &pointers, ""), signer)
}

/// Austria's list: the provider of `trusted_list::tests`, its granted `CA/QC` service.
fn member(signer: Key) -> String {
    member_with(signer, &[GRANTED_AT_LATER])
}

/// Austria's list with `services` under one provider, signed by `signer`.
fn member_with(signer: Key, services: &[Written<'_>]) -> String {
    let providers = list_xml("AT", &[("A provider", services)]);
    let start = providers
        .find("<tsl:TrustServiceProviderList>")
        .expect("a provider list");
    let end = providers
        .find("</tsl:TrustServiceStatusList>")
        .expect("a closed list");
    let providers = providers.get(start..end).expect("the provider list");
    signed(&scheme("AT", &[], "", providers), signer)
}

fn certificate_file() -> (String, Vec<u8>) {
    (format!("{OJEU_LOTL_CERTS_SET}.cer"), LOTL_KEY.certificate())
}

fn read(
    files: Vec<(&str, String)>,
    extra: Vec<(String, Vec<u8>)>,
    at: i64,
) -> super::super::Reading {
    let mut supplied: Vec<(String, Vec<u8>)> = files
        .into_iter()
        .map(|(name, text)| (name.to_owned(), text.into_bytes()))
        .collect();
    supplied.extend(extra);
    Lists::of(supplied, "the tests".to_owned()).read(Instant::from_unix_seconds(at))
}

/// The arrangement both clauses pass on: the LOTL names the Official Journal's URI, its own `EU`
/// pointer names its signer, the Official Journal's set holds that signer, and Austria's list is
/// signed by the certificate the LOTL's `AT` pointer names.
fn passing() -> (String, String) {
    (
        lotl(&[OJEU_LOC_AT_PUBLICATION], &[LOTL_KEY], LOTL_KEY),
        member(MEMBER_KEY),
    )
}

fn refused_with(reading: &super::super::Reading, name: &str) -> Vec<String> {
    match reading.refused.iter().find(|(file, _)| file == name) {
        Some((_, TrustedListError::NotAuthenticated(outcome))) => {
            let mut all = vec![outcome.status.to_owned()];
            all.extend(outcome.sub_status.iter().cloned());
            all
        }
        other => panic!("{name} refused as not authenticated, not {other:?}"),
    }
}

/// PRO-4.1.4-14 and -15, then PRO-4.2.4-11 and -12: the LOTL passes, Austria's list passes under
/// it, is the one list searched, and carries the LOTL's sequence number; the LOTL is clause 4.1's
/// input and not searched itself.
#[test]
fn a_list_signed_by_the_certificate_the_authenticated_lotl_names_is_authenticated() {
    let (lotl, member) = passing();
    let reading = read(
        vec![("eu-lotl.xml", lotl), ("at.xml", member)],
        vec![certificate_file()],
        NOW,
    );
    assert_eq!(
        reading.lotl,
        Some((
            7,
            Outcome {
                status: "LOTL_VERIFICATION_PASSED",
                sub_status: Vec::new()
            }
        ))
    );
    assert!(reading.refused.is_empty(), "{:?}", reading.refused);
    let [list] = reading.lists.as_slice() else {
        panic!("Austria's list alone: {:?}", reading.lists);
    };
    assert_eq!(list.territory, "AT");
    assert_eq!(
        list.authentication,
        Authentication::Authenticated {
            lotl_sequence_number: 7
        }
    );
}

/// The plant on clause 4.2: Austria's list changed after signing fails PRO-4.2.4-09 and is not
/// searched.
#[test]
fn a_member_list_changed_after_signing_is_refused() {
    let (lotl, member) = passing();
    let changed = member.replace("A provider", "B provider");
    let reading = read(
        vec![("eu-lotl.xml", lotl), ("at.xml", changed)],
        vec![certificate_file()],
        NOW,
    );
    assert!(reading.lists.is_empty());
    let indications = refused_with(&reading, "at.xml");
    assert_eq!(
        indications.first().map(String::as_str),
        Some("EUTL_VERIFICATION_FAILED")
    );
    assert!(
        indications[1].starts_with("EUTL_SIGNATURE_VERIFICATION_FAILED"),
        "{indications:?}"
    );
}

/// PRO-4.2.4-07: a list signed by a certificate the LOTL's pointer for its territory does not name.
#[test]
fn a_member_list_signed_by_another_certificate_is_not_authenticated_by_the_lotl() {
    let (lotl, _) = passing();
    let reading = read(
        vec![("eu-lotl.xml", lotl), ("at.xml", member(LOTL_KEY))],
        vec![certificate_file()],
        NOW,
    );
    assert_eq!(
        refused_with(&reading, "at.xml"),
        [
            "EUTL_VERIFICATION_FAILED",
            "EUTLSO_SIGNER_CERT_NOT_AUTHENTICATED_BY_LOTL"
        ]
    );
}

/// PRO-4.1.4-12, and PRO-4.2.4-02 after it: a LOTL whose signer is not in the Official Journal's
/// set authenticates no list.
#[test]
fn a_lotl_signer_outside_the_official_journals_set_authenticates_nothing() {
    let (lotl, member) = passing();
    let reading = read(
        vec![("eu-lotl.xml", lotl), ("at.xml", member)],
        vec![(
            format!("{OJEU_LOTL_CERTS_SET}.cer"),
            MEMBER_KEY.certificate(),
        )],
        NOW,
    );
    let expected = "PIVOT_n_LOTLSO_SIGNER_CERT_NOT_AUTHENTICATED_BY_OJEU";
    assert_eq!(
        reading
            .lotl
            .as_ref()
            .map(|(_, outcome)| outcome.sub_status.clone()),
        Some(vec![expected.to_owned()])
    );
    assert_eq!(
        refused_with(&reading, "at.xml"),
        [
            "TL_VERIFICATION_FAILED",
            "LOTL_VERIFICATION_FAILED",
            expected
        ]
    );
}

/// PRO-4.1.4-03, -10 a) and -13, each the one input changed.
#[test]
fn the_lotls_own_indications_are_given() {
    let lotl_indication = |files: Vec<(&str, String)>, at: i64| {
        read(files, vec![certificate_file()], at)
            .lotl
            .map(|(_, outcome)| outcome.sub_status)
    };
    let elsewhere = lotl(
        &["https://example.org/another-journal"],
        &[LOTL_KEY],
        LOTL_KEY,
    );
    assert_eq!(
        lotl_indication(vec![("eu-lotl.xml", elsewhere)], NOW),
        Some(vec![
            "OJEU_LOCATION_INPUT_NOT_MATCHING_OJEU_LOCATION_IN_LOTL".to_owned()
        ])
    );
    let not_its_own = lotl(&[OJEU_LOC_AT_PUBLICATION], &[MEMBER_KEY], LOTL_KEY);
    assert_eq!(
        lotl_indication(vec![("eu-lotl.xml", not_its_own)], NOW),
        Some(vec![
            "LOTL_SIGNER_CERT_NOT_AUTHENTICATED_BY_LOTL".to_owned()
        ])
    );
    let (passing_lotl, _) = passing();
    assert_eq!(
        lotl_indication(vec![("eu-lotl.xml", passing_lotl)], LATE),
        Some(vec!["LOTL_NEXTUPDATE_PASSED".to_owned()])
    );
}

/// PRO-4.1.4-11: one pivot between the LOTL and the Official Journal. The pivot's `EU` pointer
/// names the LOTL's signer (c) and its own (g), and its own signer is the Official Journal's (12).
/// Without the pivot's file, the procedure cannot obtain it and says so.
#[test]
fn a_pivot_carries_the_official_journals_certificate_to_the_lotl() {
    let pivot_uri = "https://ec.europa.eu/tools/lotl/eu-lotl-pivot-1.xml";
    let current = lotl(&[pivot_uri, OJEU_LOC_AT_PUBLICATION], &[LOTL_KEY], LOTL_KEY);
    let pivot = lotl(
        &[OJEU_LOC_AT_PUBLICATION],
        &[LOTL_KEY, MEMBER_KEY],
        MEMBER_KEY,
    );
    let journal = vec![(
        format!("{OJEU_LOTL_CERTS_SET}.cer"),
        MEMBER_KEY.certificate(),
    )];
    let reading = read(
        vec![
            ("eu-lotl.xml", current.clone()),
            ("eu-lotl-pivot-1.xml", pivot),
            ("at.xml", member(MEMBER_KEY)),
        ],
        journal.clone(),
        NOW,
    );
    assert_eq!(
        reading.lotl.as_ref().map(|(_, outcome)| outcome.status),
        Some("LOTL_VERIFICATION_PASSED")
    );
    assert_eq!(reading.lists.len(), 1, "{:?}", reading.refused);
    let reading = read(vec![("eu-lotl.xml", current)], journal, NOW);
    assert_eq!(
        reading.lotl.map(|(_, outcome)| outcome.sub_status),
        Some(vec![format!("{pivot_uri} NOT_SUPPLIED")])
    );
}

/// Where the host supplied neither the LOTL nor a parameter of GPR-4.0-02's, nothing is
/// authenticated and the list is used on the host's word, as before the procedures existed.
#[test]
fn without_a_lotl_or_the_official_journals_certificates_a_list_is_vouched_for() {
    let reading = read(vec![("at.xml", member(MEMBER_KEY))], Vec::new(), NOW);
    assert_eq!(reading.lotl, None);
    let [list] = reading.lists.as_slice() else {
        panic!("one list");
    };
    assert_eq!(list.authentication, Authentication::VouchedForByTheHost);
}

/// RFC 7468's PEM armour is read for the Official Journal's set as DER is.
#[test]
fn the_official_journals_set_reads_from_pem() {
    let pem = format!(
        "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----\n",
        base64_of(&LOTL_KEY.certificate())
    );
    assert_eq!(
        certificates_in(pem.as_bytes()),
        vec![LOTL_KEY.certificate()]
    );
    assert_eq!(file_name(OJEU_LOTL_LOC_AT_PUBLICATION), "eu-lotl.xml");
}

/// The arrangement both clauses pass on, Austria's list holding `services`, read at 2026-11-12:
/// what a verdict built on an authenticated list is computed from.
pub(crate) fn authenticated_reading(services: &[Written<'_>]) -> super::super::Reading {
    read(
        vec![
            (
                "eu-lotl.xml",
                lotl(&[OJEU_LOC_AT_PUBLICATION], &[LOTL_KEY], LOTL_KEY),
            ),
            ("at.xml", member_with(MEMBER_KEY, services)),
        ],
        vec![certificate_file()],
        NOW,
    )
}
