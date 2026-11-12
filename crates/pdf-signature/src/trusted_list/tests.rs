//! TS 119 612's list read, and TS 119 615 clause 4.3's search run over it, one requirement at a
//! time.
//!
//! Every list here is written by these tests, with the self-signed certificate
//! [`crate::x509_fixtures::CERTIFICATE`] as the service digital identity, so the expected values
//! are the clauses' and not any published list's: a real list would pin what a supervisory body
//! wrote this month, not what the procedure does. The path half of PRO-4.3.4-03 ii) — an
//! identity that *issued* the certificate rather than being it — is exercised by
//! `crate::verdict::tests`, which holds the signed document and the root that issued its signer.

use super::*;
use crate::x509_fixtures::{CERTIFICATE, hex};

/// 2020-01-01T00:00:00Z.
const EARLIER: i64 = 1_577_836_800;
/// 2024-01-01T00:00:00Z.
const LATER: i64 = 1_704_067_200;
/// 2026-11-01T00:00:00Z, the fixture lists' issue instant.
const ISSUED: i64 = 1_793_491_200;
/// 2026-12-01T00:00:00Z, their next update.
const NEXT: i64 = 1_796_083_200;

/// RFC 4648 section 4's alphabet, padded — what TS 119 612 clause 5.5.3's `X509Certificate`
/// holds.
pub(crate) fn base64_of(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let mut padded = [0_u8; 3];
        padded[..chunk.len()].copy_from_slice(chunk);
        let word = u32::from_be_bytes([0, padded[0], padded[1], padded[2]]);
        for (at, shift) in [18_u32, 12, 6, 0].into_iter().enumerate() {
            if at <= chunk.len() {
                out.push(char::from(ALPHABET[(word >> shift & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// One `TSPService`: its type, the current status and its instant, the history instances (each a
/// status and an instant), the `additionalServiceInformation` URI, and the identity's DER.
#[derive(Clone, Copy)]
pub(crate) struct Written<'a> {
    pub(crate) service_type: &'a str,
    pub(crate) status: &'a str,
    pub(crate) since: &'a str,
    pub(crate) history: &'a [(&'a str, &'a str)],
    pub(crate) for_type: Option<&'a str>,
    pub(crate) identity: Option<&'a [u8]>,
}

pub(crate) fn service_xml(service: &Written<'_>) -> String {
    let identity = base64_of(
        &service
            .identity
            .map_or_else(|| hex(CERTIFICATE), <[u8]>::to_vec),
    );
    let extensions = service.for_type.map_or_else(String::new, |uri| {
        format!(
            "<tsl:ServiceInformationExtensions><tsl:Extension Critical=\"true\">\
             <tsl:AdditionalServiceInformation><tsl:URI xml:lang=\"en\">{uri}</tsl:URI>\
             </tsl:AdditionalServiceInformation></tsl:Extension></tsl:ServiceInformationExtensions>"
        )
    });
    let history = service
        .history
        .iter()
        .map(|(status, since)| {
            format!(
                "<tsl:ServiceHistoryInstance>\
                 <tsl:ServiceTypeIdentifier>{}</tsl:ServiceTypeIdentifier>\
                 <tsl:ServiceName><tsl:Name xml:lang=\"en\">An earlier name</tsl:Name></tsl:ServiceName>\
                 <tsl:ServiceDigitalIdentity><tsl:DigitalId><tsl:X509SKI>AAAA</tsl:X509SKI></tsl:DigitalId></tsl:ServiceDigitalIdentity>\
                 <tsl:ServiceStatus>{status}</tsl:ServiceStatus>\
                 <tsl:StatusStartingTime>{since}</tsl:StatusStartingTime>{extensions}\
                 </tsl:ServiceHistoryInstance>",
                service.service_type
            )
        })
        .collect::<Vec<String>>()
        .concat();
    let history = if history.is_empty() {
        history
    } else {
        format!("<tsl:ServiceHistory>{history}</tsl:ServiceHistory>")
    };
    format!(
        "<tsl:TSPService><tsl:ServiceInformation>\
         <tsl:ServiceTypeIdentifier>{}</tsl:ServiceTypeIdentifier>\
         <tsl:ServiceName><tsl:Name xml:lang=\"en\">Qualified certificates &amp; seals</tsl:Name></tsl:ServiceName>\
         <tsl:ServiceDigitalIdentity><tsl:DigitalId><tsl:X509Certificate>\n{identity}\n</tsl:X509Certificate></tsl:DigitalId></tsl:ServiceDigitalIdentity>\
         <tsl:ServiceStatus>{}</tsl:ServiceStatus>\
         <tsl:StatusStartingTime>{}</tsl:StatusStartingTime>{extensions}\
         </tsl:ServiceInformation>{history}</tsl:TSPService>",
        service.service_type, service.status, service.since
    )
}

/// A list of `territory` with one provider per entry of `providers`, each its name and services.
pub(crate) fn list_xml(territory: &str, providers: &[(&str, &[Written<'_>])]) -> String {
    let providers = providers
        .iter()
        .map(|(name, services)| {
            let services: String = services.iter().map(service_xml).collect();
            format!(
                "<tsl:TrustServiceProvider><tsl:TSPInformation>\
                 <tsl:TSPName><tsl:Name xml:lang=\"en\">{name}</tsl:Name></tsl:TSPName>\
                 </tsl:TSPInformation><tsl:TSPServices>{services}</tsl:TSPServices>\
                 </tsl:TrustServiceProvider>"
            )
        })
        .collect::<Vec<String>>()
        .concat();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <tsl:TrustServiceStatusList xmlns:tsl=\"{NAMESPACE}\" TSLTag=\"http://uri.etsi.org/19612/TSLTag\">\
         <tsl:SchemeInformation><tsl:TSLVersionIdentifier>6</tsl:TSLVersionIdentifier>\
         <tsl:TSLSequenceNumber>42</tsl:TSLSequenceNumber>\
         <tsl:SchemeTerritory>{territory}</tsl:SchemeTerritory>\
         <tsl:ListIssueDateTime>2026-11-01T00:00:00Z</tsl:ListIssueDateTime>\
         <tsl:NextUpdate><tsl:dateTime>2026-12-01T00:00:00Z</tsl:dateTime></tsl:NextUpdate>\
         </tsl:SchemeInformation>\
         <tsl:TrustServiceProviderList>{providers}</tsl:TrustServiceProviderList>\
         </tsl:TrustServiceStatusList>"
    )
}

/// A `CA/QC` service granted from [`LATER`], under supervision from [`EARLIER`] before that, for
/// e-signatures.
pub(crate) const GRANTED_AT_LATER: Written<'static> = Written {
    service_type: CA_QC,
    status: GRANTED,
    since: "2024-01-01T00:00:00Z",
    history: &[(
        "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/undersupervision",
        "2020-01-01T00:00:00Z",
    )],
    for_type: Some(FOR_TYPES[0]),
    identity: None,
};

fn read(xml: &str) -> TrustedList {
    TrustedList::read(xml.as_bytes()).expect("the fixture list reads")
}

/// TS 119 612 clause 5.3's scheme fields and clauses 5.4 to 5.6's provider and service fields,
/// as written, with clause 5.1.3's date-times as instants and clause 5.5.3's certificate decoded.
#[test]
fn a_list_reads_as_clause_5_writes_it() {
    let list = read(&list_xml("AT", &[("A provider", &[GRANTED_AT_LATER])]));
    assert_eq!(list.territory, "AT");
    assert_eq!(list.sequence_number, 42);
    assert_eq!(list.issued.unix_seconds(), ISSUED);
    assert_eq!(list.next_update.map(Instant::unix_seconds), Some(NEXT));
    let [provider] = list.providers.as_slice() else {
        panic!("one provider: {:?}", list.providers);
    };
    assert_eq!(provider.name, "A provider");
    let [service] = provider.services.as_slice() else {
        panic!("one service");
    };
    assert_eq!(service.current.name, "Qualified certificates & seals");
    assert_eq!(service.current.certificates, vec![hex(CERTIFICATE)]);
    assert_eq!(service.current.since.unix_seconds(), LATER);
    assert_eq!(service.current.additional_information, vec![FOR_TYPES[0]]);
    let [previous] = service.history.as_slice() else {
        panic!("one history instance");
    };
    assert_eq!(previous.since.unix_seconds(), EARLIER);
    assert!(
        previous.certificates.is_empty(),
        "clause 5.6.3: no certificate"
    );
}

/// Clause 5.3.15: an empty next update is a closed list, not a malformed one.
#[test]
fn an_empty_next_update_is_a_closed_list() {
    let xml = list_xml("AT", &[]).replace("2026-12-01T00:00:00Z", "");
    assert_eq!(read(&xml).next_update, None);
}

/// What is refused, by name: a field clause 5 requires, a root that is not a list, a document
/// type declaration, and a list past the octet bound.
#[test]
fn what_is_not_a_list_is_refused_by_name() {
    let xml = list_xml("AT", &[]).replace("<tsl:SchemeTerritory>AT</tsl:SchemeTerritory>", "");
    assert_eq!(
        TrustedList::read(xml.as_bytes()),
        Err(TrustedListError::Malformed("SchemeTerritory"))
    );
    let other = format!("<tsl:Other xmlns:tsl=\"{NAMESPACE}\"/>");
    assert_eq!(
        TrustedList::read(other.as_bytes()),
        Err(TrustedListError::NotATrustedList)
    );
    let typed = format!("<!DOCTYPE x [<!ENTITY a \"b\">]>{}", list_xml("AT", &[]));
    assert!(matches!(
        TrustedList::read(typed.as_bytes()),
        Err(TrustedListError::Xml(_))
    ));
    let huge = vec![b' '; MAX_LIST_OCTETS + 1];
    assert!(matches!(
        TrustedList::read(&huge),
        Err(TrustedListError::TooLarge { .. })
    ));
}

/// The search over `list` for [`CERTIFICATE`] at `seconds`.
fn search(list: &TrustedList, seconds: i64) -> Matching<'_> {
    let der = hex(CERTIFICATE);
    let certificate = x509::parse(&der).expect("the fixture parses");
    matching(
        list,
        &certificate,
        &[],
        CA_QC,
        Instant::from_unix_seconds(seconds),
    )
}

/// PRO-4.3.4-01: a territory that is not a Member State, `GB` or `UK` fails with
/// `Provided_country_code_not_representing_an_EU_MS` and matches nothing.
#[test]
fn a_list_outside_the_union_fails_clause_4_3() {
    let list = read(&list_xml("US", &[("A provider", &[GRANTED_AT_LATER])]));
    let matched = search(&list, LATER);
    assert_eq!(matched.status, Status::Failed);
    assert_eq!(matched.faults, vec![Fault::NotAnEuMemberState]);
    assert!(matched.results.is_empty());
}

/// PRO-4.3.4-03 i) and ii): a service of the type asked for whose identity is the certificate's
/// own key and subject matches, and one of another type does not. And b): `SI-at-Date-time` is
/// the current information from its starting instant on — that instant included — the history
/// instance in force before it, and nothing before the first status.
#[test]
fn the_service_and_its_status_at_the_instant_are_found() {
    let pkc = Written {
        service_type: "http://uri.etsi.org/TrstSvc/Svctype/CA/PKC",
        ..GRANTED_AT_LATER
    };
    let list = read(&list_xml("DE", &[("A provider", &[GRANTED_AT_LATER, pkc])]));

    let current = search(&list, LATER);
    assert_eq!(current.status, Status::Passed);
    let [matched] = current.results.as_slice() else {
        panic!("only the CA/QC service: {:?}", current.results);
    };
    assert_eq!(matched.provider.name, "A provider");
    assert_eq!(
        matched.at_date_time.map(|state| state.status.as_str()),
        Some(GRANTED)
    );

    let between = search(&list, LATER - 1);
    assert_eq!(
        between
            .results
            .first()
            .and_then(|matched| matched.at_date_time)
            .map(|state| state.since.unix_seconds()),
        Some(EARLIER),
        "the history instance in force a second before the current status"
    );
    let before = search(&list, EARLIER - 1);
    assert_eq!(
        before.results.first().map(|matched| matched.at_date_time),
        Some(None)
    );
}

/// PRO-4.3.4-03A: history instances not in descending order, or two statuses sharing an instant,
/// stop the process with the error reported.
#[test]
fn a_misordered_history_stops_the_search() {
    let out_of_order: &[(&str, &str)] = &[
        (GRANTED, "2020-01-01T00:00:00Z"),
        (GRANTED, "2022-01-01T00:00:00Z"),
    ];
    let shared: &[(&str, &str)] = &[(GRANTED, "2024-01-01T00:00:00Z")];
    for (history, fault) in [
        (out_of_order, Fault::HistoryOutOfOrder),
        (shared, Fault::HistorySharesAnInstant),
    ] {
        let service = Written {
            history,
            ..GRANTED_AT_LATER
        };
        let list = read(&list_xml("FR", &[("A provider", &[service])]));
        let matched = search(&list, LATER);
        assert_eq!(matched.status, Status::Failed);
        assert_eq!(matched.faults, vec![fault]);
    }
}

/// PRO-4.3.4-05 and -06: two matched services both for e-signatures warn where their statuses
/// agree and are an error where they differ; PRO-4.3.4-11: two providers fail the process.
#[test]
fn duplicates_and_conflicting_providers_are_reported() {
    let same = read(&list_xml(
        "IT",
        &[("A provider", &[GRANTED_AT_LATER, GRANTED_AT_LATER])],
    ));
    let matched = search(&same, LATER);
    assert_eq!(matched.status, Status::Passed);
    assert_eq!(matched.faults, vec![Fault::DuplicationWarning(0)]);

    let withdrawn = Written {
        status: "http://uri.etsi.org/TrstSvc/TrustedList/Svcstatus/withdrawn",
        ..GRANTED_AT_LATER
    };
    let differ = read(&list_xml(
        "IT",
        &[("A provider", &[GRANTED_AT_LATER, withdrawn])],
    ));
    assert_eq!(
        search(&differ, LATER).faults,
        vec![Fault::DuplicationError(0)]
    );

    let two = read(&list_xml(
        "IT",
        &[
            ("A provider", &[GRANTED_AT_LATER]),
            ("Another", &[GRANTED_AT_LATER]),
        ],
    ));
    let matched = search(&two, LATER);
    assert_eq!(matched.status, Status::Failed);
    assert!(matched.faults.contains(&Fault::ProviderConflict));
}
