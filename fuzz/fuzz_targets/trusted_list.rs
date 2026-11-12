//! Fuzzes ETSI TS 119 612's trusted list reader (`pdf_signature::trusted_list`, ADR 2834) and TS
//! 119 615 clause 4.3's search over what it reads.
//!
//! A trusted list reaches the reader as octets a host read from a directory a person named, so
//! they are a stranger's XML: the octet, entry, nesting and service bounds (`MAX_LIST_OCTETS`,
//! `MAX_ENTRIES`, `MAX_SERVICES`) are what stand between a hostile list and the heap, and a
//! document type declaration is refused before any entity could expand. Beyond never panicking —
//! overflow checks stay on in this profile — two properties are under test:
//!
//! - **The reading is a function of the bytes**: read twice, the same list or the same refusal.
//! - **The search never reaches past a list's services**: every tuple clause 4.3 returns names a
//!   service of the list it searched, with `SI-at-Date-time` the service's current information or
//!   one of its own history instances, at any instant.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_signature::trusted_list::{CA_QC, TrustedList, matching};
use pdf_signature::x509::{self, Instant};

fuzz_target!(|data: &[u8]| {
    // The first eight bytes are the instant the search asks about, and the rest is the list.
    let (instant, octets) = data.split_at(data.len().min(8));
    let mut seconds = [0_u8; 8];
    seconds[..instant.len()].copy_from_slice(instant);
    let at = Instant::from_unix_seconds(i64::from_le_bytes(seconds));

    let first = TrustedList::read(octets);
    assert_eq!(
        first,
        TrustedList::read(octets),
        "the reading is a function of the bytes"
    );
    let Ok(list) = first else {
        return;
    };
    // Every identity the list states is offered as the certificate searched for, so the
    // identity-equals-certificate branch and the path branch both run on the fuzzer's bytes.
    let identities: Vec<&Vec<u8>> = list
        .providers
        .iter()
        .flat_map(|provider| &provider.services)
        .flat_map(|service| &service.current.certificates)
        .take(4)
        .collect();
    for der in identities {
        let Ok(target) = x509::parse(der) else {
            continue;
        };
        let found = matching(&list, &target, &[], CA_QC, at);
        for matched in &found.results {
            assert!(
                list.providers
                    .iter()
                    .flat_map(|provider| &provider.services)
                    .any(|service| std::ptr::eq(service, matched.service)),
                "a tuple names a service of the list searched"
            );
            if let Some(state) = matched.at_date_time {
                assert!(
                    std::ptr::eq(state, &raw const matched.service.current)
                        || matched
                            .service
                            .history
                            .iter()
                            .any(|h| std::ptr::eq(h, state)),
                    "SI-at-Date-time is the service's own"
                );
            }
        }
    }
});
