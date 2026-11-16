//! A refresh through a fixture transport: the files TS 119 615 clauses 4.1 and 4.2 obtain, served
//! from a map of locations to octets written and signed here, and what is kept, removed and
//! updated, one input changed at a time.
//!
//! The arrangement clause 4.1 passes on is `authentication::tests`': the list of trusted lists
//! (territory `EU`) signed by [`LOTL_KEY`], whose certificate is the Official Journal's set, and
//! two Member States' lists signed by [`MEMBER_KEY`], the certificate the LOTL's pointers name for
//! `AT` and `BE`. Both Member States publish under a location ending `tsl.xml`, as three did in the
//! LOTL of 2026-11-12.

use super::super::authentication::tests::{pointer, scheme};
use super::super::authentication::{OJEU_LOC_AT_PUBLICATION, OJEU_LOTL_LOC_AT_PUBLICATION};
use super::super::tests::{GRANTED_AT_LATER, list_xml};
use super::*;
use crate::xml_signature::tests::{Key, LOTL_KEY, MEMBER_KEY, signed};

/// 2026-11-12T00:00:00Z, before the fixtures' next update.
const NOW: i64 = 1_794_441_600;
const AT_TSL: &str = "https://example.at/tsl.xml";
const BE_TSL: &str = "https://example.be/tsl.xml";
const OJ_2026: &str = "https://example.eu/oj/C-2026-1944";
const PIVOT: &str = "https://example.eu/lotl/eu-lotl-pivot-9.xml";
const CERTIFICATES: &str = "OJEU-LOTL-Certs-Set.cer";

/// A LOTL of scheme information URIs `uris`, its own pointer at `self_location` naming
/// [`LOTL_KEY`], and `AT` and `BE` pointers naming [`MEMBER_KEY`], signed by `signer`.
fn lotl_at(uris: &[&str], self_location: &str, signer: Key) -> String {
    let pointers = pointer("EU", self_location, &[LOTL_KEY])
        + &pointer("AT", AT_TSL, &[MEMBER_KEY])
        + &pointer("BE", BE_TSL, &[MEMBER_KEY]);
    signed(&scheme("EU", uris, &pointers, ""), signer)
}

fn lotl(signer: Key) -> String {
    lotl_at(
        &[OJEU_LOC_AT_PUBLICATION],
        OJEU_LOTL_LOC_AT_PUBLICATION,
        signer,
    )
}

/// The Member State `territory`'s list: one provider with a granted `CA/QC` service.
fn member(territory: &str, signer: Key) -> String {
    let providers = list_xml(territory, &[("A provider", &[GRANTED_AT_LATER])]);
    let start = providers
        .find("<tsl:TrustServiceProviderList>")
        .expect("a provider list");
    let end = providers
        .find("</tsl:TrustServiceStatusList>")
        .expect("a closed list");
    let providers = providers.get(start..end).expect("the provider list");
    signed(&scheme(territory, &[], "", providers), signer)
}

/// The host's files: the Official Journal's certificate, and `held`.
fn host(held: Vec<(String, Vec<u8>)>) -> Lists {
    let mut files = vec![(CERTIFICATES.to_owned(), LOTL_KEY.certificate())];
    files.extend(held);
    Lists::of(files, "the tests".to_owned())
}

/// A transport serving `served`, recording every location it was asked for.
fn run(lists: &Lists, served: &[(&str, String)]) -> (Result<Refresh, RefreshRefusal>, Vec<String>) {
    let mut asked = Vec::new();
    let mut obtain = |uri: &str| {
        asked.push(uri.to_owned());
        served
            .iter()
            .find(|(location, _)| *location == uri)
            .map(|(_, text)| text.clone().into_bytes())
            .ok_or_else(|| format!("{uri} answered 404"))
    };
    let refreshed = refresh(lists, Instant::from_unix_seconds(NOW), &mut obtain);
    (refreshed, asked)
}

fn fate_of<'r>(refreshed: &'r Refresh, uri: &str) -> &'r Fate {
    &refreshed
        .locations
        .iter()
        .find(|location| location.uri == uri)
        .unwrap_or_else(|| panic!("{uri} among the locations"))
        .fate
}

fn stored<'r>(refreshed: &'r Refresh, name: &str) -> Option<&'r [u8]> {
    refreshed
        .store
        .iter()
        .find(|(file, _)| file == name)
        .map(|(_, octets)| octets.as_slice())
}

/// RFC 3986 section 2.1's percent-encoding of every octet outside section 2.3's unreserved set,
/// in upper-case hexadecimal, so the two Member States' `tsl.xml` are two names.
#[test]
fn a_stored_name_is_the_location_percent_encoded() {
    assert_eq!(
        stored_name(OJEU_LOTL_LOC_AT_PUBLICATION),
        "https%3A%2F%2Fec.europa.eu%2Ftools%2Flotl%2Feu-lotl.xml"
    );
    assert_eq!(stored_name("a~b_c-d.e f?g"), "a~b_c-d.e%20f%3Fg");
    assert_ne!(stored_name(AT_TSL), stored_name(BE_TSL));
}

/// PRO-4.1.4-01, then PRO-4.2.4-03 and -04 for each territory: the LOTL and both Member States'
/// lists are fetched and kept under their locations, and a reading of what was stored
/// authenticates both lists by clause 4.2 — neither overwrote the other.
#[test]
fn what_both_clauses_authenticate_is_kept_under_its_location() {
    let lists = host(Vec::new());
    let (refreshed, asked) = run(
        &lists,
        &[
            (OJEU_LOTL_LOC_AT_PUBLICATION, lotl(LOTL_KEY)),
            (AT_TSL, member("AT", MEMBER_KEY)),
            (BE_TSL, member("BE", MEMBER_KEY)),
        ],
    );
    let refreshed = refreshed.expect("a refresh");
    assert_eq!(asked, [OJEU_LOTL_LOC_AT_PUBLICATION, AT_TSL, BE_TSL]);
    assert_eq!(refreshed.lotl.status, "LOTL_VERIFICATION_PASSED");
    for uri in [OJEU_LOTL_LOC_AT_PUBLICATION, AT_TSL, BE_TSL] {
        assert_eq!(fate_of(&refreshed, uri), &Fate::Kept, "{uri}");
        assert!(stored(&refreshed, &stored_name(uri)).is_some(), "{uri}");
    }
    assert!(refreshed.remove.is_empty());
    assert!(refreshed.configuration.is_empty());
    let mut files = vec![(CERTIFICATES.to_owned(), LOTL_KEY.certificate())];
    files.extend(refreshed.store);
    let reading = Lists::of(files, "stored".to_owned()).read(Instant::from_unix_seconds(NOW));
    let territories: Vec<&str> = reading
        .lists
        .iter()
        .map(|list| list.territory.as_str())
        .collect();
    assert_eq!(territories.len(), 2, "{reading:?}");
    assert!(territories.contains(&"AT") && territories.contains(&"BE"));
    assert!(reading.lists.iter().all(|list| matches!(
        list.authentication,
        super::super::Authentication::Authenticated { .. }
    )));
}

/// PRO-4.2.4-07: a Member State's list whose signer the LOTL's pointer does not name is not kept,
/// and the copy the host held of it stays rather than being removed.
#[test]
fn a_member_list_clause_4_2_refuses_is_not_kept_and_the_held_copy_stays() {
    let held = (stored_name(AT_TSL), member("AT", MEMBER_KEY).into_bytes());
    let lists = host(vec![held]);
    let (refreshed, _) = run(
        &lists,
        &[
            (OJEU_LOTL_LOC_AT_PUBLICATION, lotl(LOTL_KEY)),
            (AT_TSL, member("AT", LOTL_KEY)),
            (BE_TSL, member("BE", MEMBER_KEY)),
        ],
    );
    let refreshed = refreshed.expect("a refresh");
    let Fate::NotAuthenticated(outcome) = fate_of(&refreshed, AT_TSL) else {
        panic!("AT refused, not {:?}", fate_of(&refreshed, AT_TSL));
    };
    assert_eq!(
        outcome.sub_status,
        ["EUTLSO_SIGNER_CERT_NOT_AUTHENTICATED_BY_LOTL"]
    );
    assert!(stored(&refreshed, &stored_name(AT_TSL)).is_none());
    assert!(refreshed.remove.is_empty());
    assert_eq!(fate_of(&refreshed, BE_TSL), &Fate::Kept);
}

/// PRO-4.1.4-12 and PRO-4.2.4-02: a LOTL signed outside the Official Journal's set keeps nothing,
/// and no Member State's list is asked for.
#[test]
fn a_lotl_clause_4_1_refuses_keeps_nothing_and_asks_for_no_member_list() {
    let lists = host(Vec::new());
    let (refreshed, asked) = run(
        &lists,
        &[
            (OJEU_LOTL_LOC_AT_PUBLICATION, lotl(MEMBER_KEY)),
            (AT_TSL, member("AT", MEMBER_KEY)),
        ],
    );
    let refreshed = refreshed.expect("a refresh");
    assert_eq!(refreshed.lotl.status, "LOTL_VERIFICATION_FAILED");
    assert_eq!(asked, [OJEU_LOTL_LOC_AT_PUBLICATION]);
    assert!(refreshed.store.is_empty() && refreshed.remove.is_empty());
    assert!(matches!(
        fate_of(&refreshed, OJEU_LOTL_LOC_AT_PUBLICATION),
        Fate::NotAuthenticated(_)
    ));
}

/// GPR-4.0-02's `OJEU-LOTL-Certs-Set` has no default, so without one PRO-4.1.4-12 could
/// authenticate nothing: the transport is not asked at all.
#[test]
fn without_the_official_journals_certificates_nothing_is_asked_for() {
    let lists = Lists::of(Vec::new(), "the tests".to_owned());
    let (refreshed, asked) = run(&lists, &[(OJEU_LOTL_LOC_AT_PUBLICATION, lotl(LOTL_KEY))]);
    assert_eq!(
        refreshed,
        Err(RefreshRefusal::NoOfficialJournalCertificates)
    );
    assert!(asked.is_empty());
}

/// A list that could not be fetched keeps the host's copy; a file the host stored from a location
/// the authenticated LOTL no longer points at is removed, and a file a person named by hand is
/// not.
#[test]
fn a_list_not_fetched_stays_and_one_no_longer_pointed_at_is_removed() {
    let gone = "https://example.cz/tsl.xml";
    let lists = host(vec![
        (stored_name(BE_TSL), b"held".to_vec()),
        (stored_name(gone), b"held".to_vec()),
        ("TL-DE.xml".to_owned(), b"held".to_vec()),
    ]);
    let (refreshed, _) = run(
        &lists,
        &[
            (OJEU_LOTL_LOC_AT_PUBLICATION, lotl(LOTL_KEY)),
            (AT_TSL, member("AT", MEMBER_KEY)),
        ],
    );
    let refreshed = refreshed.expect("a refresh");
    assert_eq!(
        fate_of(&refreshed, BE_TSL),
        &Fate::NotFetched(format!("{BE_TSL} answered 404"))
    );
    assert_eq!(refreshed.remove, [stored_name(gone)]);
}

/// PRO-4.1.4-16: the authenticated LOTL's pointer to itself names another location, so
/// `OJEU-LOTL-Loc` becomes it — fetched first under PRO-4.1.4-05, the same octets; where the
/// pointer names `OJEU-LOTL-Loc` itself, nothing is updated.
#[test]
fn the_lotls_own_new_location_becomes_the_configured_one() {
    let moved = "https://example.eu/lotl/eu-lotl.xml";
    let text = lotl_at(&[OJEU_LOC_AT_PUBLICATION], moved, LOTL_KEY);
    let lists = host(Vec::new());
    let (refreshed, asked) = run(
        &lists,
        &[
            (OJEU_LOTL_LOC_AT_PUBLICATION, text.clone()),
            (moved, text),
            (AT_TSL, member("AT", MEMBER_KEY)),
            (BE_TSL, member("BE", MEMBER_KEY)),
        ],
    );
    let refreshed = refreshed.expect("a refresh");
    assert_eq!(asked.get(1).map(String::as_str), Some(moved));
    assert_eq!(stored(&refreshed, OJEU_LOTL_LOC), Some(moved.as_bytes()));
    assert_eq!(refreshed.configuration.len(), 1);

    let (unmoved, _) = run(&lists, &[(OJEU_LOTL_LOC_AT_PUBLICATION, lotl(LOTL_KEY))]);
    assert_eq!(stored(&unmoved.expect("a refresh"), OJEU_LOTL_LOC), None);
}

/// PRO-4.1.4-17 a) and b) 1): the first `https` URI not ending `.xml` is a new publication, so
/// `OJEU-Loc` becomes it and `OJEU-LOTL-Certs-Set` becomes the certificates of the `EU` pointer of
/// the pivot that follows it — both of them, in order — replacing the host's certificate file.
#[test]
fn a_new_publication_moves_the_configuration_with_the_following_pivots_certificates() {
    let pivot = signed(
        &scheme(
            "EU",
            &[PIVOT, OJEU_LOC_AT_PUBLICATION],
            &pointer("EU", OJEU_LOTL_LOC_AT_PUBLICATION, &[LOTL_KEY, MEMBER_KEY]),
            "",
        ),
        LOTL_KEY,
    );
    let lists = host(Vec::new());
    let (refreshed, asked) = run(
        &lists,
        &[
            (
                OJEU_LOTL_LOC_AT_PUBLICATION,
                lotl_at(
                    &[OJ_2026, PIVOT, OJEU_LOC_AT_PUBLICATION],
                    OJEU_LOTL_LOC_AT_PUBLICATION,
                    LOTL_KEY,
                ),
            ),
            (PIVOT, pivot),
        ],
    );
    let refreshed = refreshed.expect("a refresh");
    assert_eq!(asked.get(1).map(String::as_str), Some(PIVOT));
    assert_eq!(refreshed.lotl.status, "LOTL_VERIFICATION_PASSED");
    assert_eq!(stored(&refreshed, OJEU_LOC), Some(OJ_2026.as_bytes()));
    let lotl_certificate = LOTL_KEY.certificate();
    let member_certificate = MEMBER_KEY.certificate();
    assert_eq!(
        stored(&refreshed, "OJEU-LOTL-Certs-Set-1.der"),
        Some(lotl_certificate.as_slice())
    );
    assert_eq!(
        stored(&refreshed, "OJEU-LOTL-Certs-Set-2.der"),
        Some(member_certificate.as_slice())
    );
    assert_eq!(refreshed.remove, [CERTIFICATES]);
}

/// The control: the new publication is followed by no pivot, so b) 1) has no certificates, and
/// neither `OJEU-Loc` nor the set is moved; the sentence says why.
#[test]
fn a_new_publication_with_no_following_pivot_moves_nothing_and_says_so() {
    let lists = host(Vec::new());
    let (refreshed, _) = run(
        &lists,
        &[(
            OJEU_LOTL_LOC_AT_PUBLICATION,
            lotl_at(
                &[
                    OJ_2026,
                    "https://example.eu/explanation.html",
                    OJEU_LOC_AT_PUBLICATION,
                ],
                OJEU_LOTL_LOC_AT_PUBLICATION,
                LOTL_KEY,
            ),
        )],
    );
    let refreshed = refreshed.expect("a refresh");
    assert_eq!(refreshed.lotl.status, "LOTL_VERIFICATION_PASSED");
    assert_eq!(stored(&refreshed, OJEU_LOC), None);
    assert!(refreshed.remove.is_empty());
    assert!(
        refreshed
            .configuration
            .first()
            .is_some_and(|sentence| sentence.contains("no pivot list follows it")),
        "{:?}",
        refreshed.configuration
    );
}

/// A file stored from the location is the one clause 4.1 reads, before a copy named by its last
/// path segment: a stale hand-saved `eu-lotl.xml` signed outside the set does not decide.
#[test]
fn a_stored_lotl_is_read_before_one_named_by_its_last_segment() {
    let lists = host(vec![
        ("eu-lotl.xml".to_owned(), lotl(MEMBER_KEY).into_bytes()),
        (
            stored_name(OJEU_LOTL_LOC_AT_PUBLICATION),
            lotl(LOTL_KEY).into_bytes(),
        ),
    ]);
    let reading = lists.read(Instant::from_unix_seconds(NOW));
    let (_, outcome) = reading.lotl.clone().expect("clause 4.1 ran");
    assert_eq!(outcome.status, "LOTL_VERIFICATION_PASSED");
    assert_eq!(
        reading
            .authenticated_lotl()
            .map(|(name, _)| name.to_owned()),
        Some(stored_name(OJEU_LOTL_LOC_AT_PUBLICATION))
    );
}
