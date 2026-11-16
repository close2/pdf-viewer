//! ETSI TS 119 172-2 clause 4.22.2.1 b)'s three steps over lists written here, one step at a time.
//!
//! Every list is this tree's own, with fixture certificates as the services' identities, so each
//! expected set is read off the clause: step 1's initial set, step 2's type filter, step 3's
//! status filter at current time, and a list of trusted lists standing for the specific lists it
//! points at.

use super::super::authentication::OJEU_LOTL_LOC_AT_PUBLICATION;
use super::super::authentication::tests::{authenticated_reading, pointer, scheme};
use super::super::tests::{GRANTED_AT_LATER, Written, list_xml};
use super::super::{CA_QC, GRANTED, Lists, NAMESPACE};
use super::*;
use crate::trust::tests::fixtures::{INTERMEDIATE, ROOT};
use crate::x509_fixtures::hex;

/// 2022-01-01T00:00:00Z: after [`GRANTED_AT_LATER`]'s supervision began, before its grant.
const SUPERVISED: Instant = Instant::from_unix_seconds(1_640_995_200);
/// 2026-06-01T00:00:00Z: after the grant.
const GRANTED_THEN: Instant = Instant::from_unix_seconds(1_780_272_000);

/// TS 119 612 clause 5.5.1.1's identifier of a qualified time-stamping service.
const QTST: &str = "http://uri.etsi.org/TrstSvc/Svctype/TSA/QTST";

/// The host's files, read with nothing to authenticate them against: every list vouched for.
fn reading(files: &[(&str, String)]) -> Reading {
    let files = files
        .iter()
        .map(|(name, text)| ((*name).to_owned(), text.clone().into_bytes()))
        .collect();
    Lists::of(files, "the tests".to_owned()).read(GRANTED_THEN)
}

fn reference(uri: &str) -> ListReference {
    ListReference {
        reference: uri.to_owned(),
        service_types: None,
        statuses: None,
    }
}

/// A `CA/QC` service granted at [`GRANTED_AT_LATER`]'s instants with `identity`.
fn ca(identity: &[u8]) -> Written<'_> {
    Written {
        identity: Some(identity),
        ..GRANTED_AT_LATER
    }
}

/// A list of trusted lists of territory `EU`, unsigned, pointing at itself and at `pointed`.
fn lotl(pointed: &[(&str, &str)]) -> String {
    let pointers = pointer("EU", "https://example.eu/lotl.xml", &[])
        + &pointed
            .iter()
            .map(|(territory, location)| pointer(territory, location, &[]))
            .collect::<String>();
    format!(
        "<tsl:TrustServiceStatusList xmlns:tsl=\"{NAMESPACE}\" \
         TSLTag=\"http://uri.etsi.org/19612/TSLTag\">{}</tsl:TrustServiceStatusList>",
        scheme("EU", &[], &pointers, "")
    )
}

/// Step 1, a specific list: every service's certificate, whatever its type or status, each once.
#[test]
fn a_specific_lists_initial_set_is_every_services_certificate() {
    let root = hex(ROOT);
    let intermediate = hex(INTERMEDIATE);
    let tsa = Written {
        service_type: QTST,
        ..ca(&intermediate)
    };
    let held = reading(&[(
        "tl.xml",
        list_xml("AT", &[("A provider", &[ca(&root), tsa, ca(&root)])]),
    )]);
    let supplied = Supplied {
        reading: &held,
        now: GRANTED_THEN,
    };
    let found = supplied
        .anchors(&reference("https://example.at/tl.xml"))
        .expect("the file named the reference's last path segment");
    assert_eq!(found.certificates, [root, intermediate]);
    assert!(found.missing.is_empty());
}

/// Step 2: only a service of a listed type contributes (Example 1's `CA/QC`).
#[test]
fn service_types_keep_only_the_services_of_those_types() {
    let root = hex(ROOT);
    let intermediate = hex(INTERMEDIATE);
    let tsa = Written {
        service_type: QTST,
        ..ca(&intermediate)
    };
    let held = reading(&[(
        "tl.xml",
        list_xml("AT", &[("A provider", &[ca(&root), tsa])]),
    )]);
    let supplied = Supplied {
        reading: &held,
        now: GRANTED_THEN,
    };
    let typed = |types: &[&str]| {
        supplied
            .anchors(&ListReference {
                service_types: Some(types.iter().map(|uri| (*uri).to_owned()).collect()),
                ..reference("https://example.at/tl.xml")
            })
            .expect("resolved")
            .certificates
    };
    assert_eq!(typed(&[CA_QC]), [root]);
    assert_eq!(typed(&[QTST]), [intermediate]);
    assert!(typed(&["http://uri.etsi.org/TrstSvc/Svctype/CA/PKC"]).is_empty());
}

/// Step 3: the status *at current time* decides (Example 2's `granted`), so the same service is
/// kept after its grant and not during its supervision.
#[test]
fn service_statuses_are_asked_at_current_time() {
    let root = hex(ROOT);
    let held = reading(&[("tl.xml", list_xml("AT", &[("A provider", &[ca(&root)])]))]);
    let granted = ListReference {
        statuses: Some(vec![GRANTED.to_owned()]),
        ..reference("https://example.at/tl.xml")
    };
    let at = |now| {
        Supplied {
            reading: &held,
            now,
        }
        .anchors(&granted)
        .expect("resolved")
        .certificates
    };
    assert_eq!(at(GRANTED_THEN), [root]);
    assert!(
        at(SUPERVISED).is_empty(),
        "under supervision, not granted, at that instant"
    );
}

/// Step 1, a list of trusted lists: the certificates of every specific list it points at, and the
/// pointed-to lists the host did not supply counted by territory.
#[test]
fn a_list_of_trusted_lists_stands_for_the_specific_lists_it_points_at() {
    let root = hex(ROOT);
    let intermediate = hex(INTERMEDIATE);
    let held = reading(&[
        (
            "lotl.xml",
            lotl(&[
                ("AT", "https://example.at/tl.xml"),
                ("BE", "https://example.be/tl.xml"),
                ("CZ", "https://example.cz/tl.xml"),
            ]),
        ),
        ("at.xml", list_xml("AT", &[("A provider", &[ca(&root)])])),
        (
            "be.xml",
            list_xml("BE", &[("Another provider", &[ca(&intermediate)])]),
        ),
    ]);
    let supplied = Supplied {
        reading: &held,
        now: GRANTED_THEN,
    };
    let found = supplied
        .anchors(&reference("https://example.eu/lotl.xml"))
        .expect("the list of trusted lists");
    assert_eq!(found.certificates, [root.clone(), intermediate]);
    assert_eq!(found.missing, ["CZ"]);
    // A specific list the list of trusted lists points at is found by that pointer's territory
    // where no file carries the reference's last path segment.
    assert_eq!(
        supplied
            .anchors(&reference("https://example.at/tl.xml"))
            .expect("Austria's, by the pointer")
            .certificates,
        [root]
    );
}

/// The same over lists TS 119 615 clauses 4.1 and 4.2 authenticated: the list of trusted lists is
/// clause 4.1's input and not searched for an issuer, and a policy may still reference it.
#[test]
fn an_authenticated_list_of_trusted_lists_can_be_referenced() {
    let root = hex(ROOT);
    let held = authenticated_reading(&[ca(&root)]);
    assert!(held.lists.iter().all(|list| list.territory != "EU"));
    let found = Supplied {
        reading: &held,
        now: GRANTED_THEN,
    }
    .anchors(&reference(OJEU_LOTL_LOC_AT_PUBLICATION))
    .expect("the authenticated list of trusted lists");
    assert_eq!(found.certificates, [root]);
    assert!(found.missing.is_empty());
}

/// A reference no supplied file answers, or one to a file the reading refused, determines nothing;
/// and with no list at all there is nothing to resolve against.
#[test]
fn a_reference_to_a_list_not_supplied_or_refused_is_unresolved() {
    let root = hex(ROOT);
    let held = reading(&[
        ("tl.xml", list_xml("AT", &[("A provider", &[ca(&root)])])),
        ("broken.xml", "<not-a-list/>".to_owned()),
    ]);
    let supplied = Supplied {
        reading: &held,
        now: GRANTED_THEN,
    };
    assert_eq!(
        supplied.anchors(&reference("https://example.fr/absent.xml")),
        Err(Unresolved::NotSupplied)
    );
    assert_eq!(
        supplied.anchors(&reference("https://example.fr/broken.xml")),
        Err(Unresolved::Refused)
    );
    let nothing = Reading::default();
    assert_eq!(
        Supplied {
            reading: &nothing,
            now: GRANTED_THEN,
        }
        .anchors(&reference("https://example.at/tl.xml")),
        Err(Unresolved::NoListSupplied)
    );
}
