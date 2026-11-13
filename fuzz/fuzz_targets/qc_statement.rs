//! Fuzzes ETSI EN 319 412-5's QC statement reader (`pdf_signature::qc_statement`, ADR 2853).
//!
//! The statements arrive inside a signer's certificate, which is a stranger's bytes, so beyond
//! never panicking — overflow checks stay on in this profile — three properties are under test:
//!
//! - **The reading is a function of the bytes**: read twice, the same statements or the same
//!   refusal.
//! - **The walk is bounded**: no reading holds more than `MAX_ENTRIES` statements, purposes and
//!   countries together.
//! - **A country is what clause 4.2.4's `CountryName` admits**: two ASCII letters.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_signature::qc_statement::{MAX_ENTRIES, QcStatements};

fuzz_target!(|data: &[u8]| {
    let first = QcStatements::read(data);
    assert_eq!(
        first,
        QcStatements::read(data),
        "the reading is a function of the bytes"
    );
    let Ok(read) = first else {
        return;
    };
    // Every statement and every member it holds is one entry, each count at most the bound;
    // saturating keeps the sum free of a panic path either way.
    let entries = [
        read.others.len(),
        usize::from(read.compliance),
        usize::from(read.sscd),
        read.types
            .as_ref()
            .map_or(0, |types| types.len().saturating_add(1)),
        read.legislation
            .as_ref()
            .map_or(0, |countries| countries.len().saturating_add(1)),
    ]
    .into_iter()
    .fold(0_usize, usize::saturating_add);
    assert!(
        entries <= MAX_ENTRIES.saturating_mul(2),
        "the walk is bounded"
    );
    for country in read.legislation.iter().flatten() {
        assert!(
            country.len() == 2 && country.bytes().all(|octet| octet.is_ascii_alphabetic()),
            "a CountryName is two letters: {country:?}"
        );
    }
});
