//! The four shipped profiles, held to the configuration format they are written in.
//!
//! `doc/profiles/*.toml` are `doc/rfc/0007` section 5a's profiles — a configuration is a list of
//! refusal sites with an answer beside each, and a profile is one this project ships. They were
//! written before the reader existed; this test is what keeps them from drifting from it again.
//! Every profile loads for every one of the six targets — `doc/rfc/0007` section 5a.2's property,
//! that a profile answers refusals and every answer leaves the file conforming, so no profile is
//! incoherent with any target — and a site or remedy a profile names wrongly fails the build here
//! rather than in an operator's pipeline.

#![expect(
    clippy::panic,
    reason = "test code: a shipped profile that does not load must fail loudly and name itself; \
              clippy.toml allows expect and panic inside #[test] fns, so only the panic in the \
              profile() helper needs the expectation"
)]

use std::path::Path;

use pdf_archive::{Flavour, Level, Target};
use pdf_transform::archive::Configuration;

const TARGETS: [Target; 6] = [
    Target::Two(Level::B),
    Target::Two(Level::U),
    Target::Two(Level::A),
    Target::Four(Flavour::Plain),
    Target::Four(Flavour::F),
    Target::Four(Flavour::E),
];

const PROFILES: [&str; 4] = [
    "refuse-any-loss",
    "as-if-printed",
    "only-metadata-loss",
    "keep-everything",
];

fn profile(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/profiles")
        .join(format!("{name}.toml"));
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
fn every_shipped_profile_loads_for_every_target() {
    for name in PROFILES {
        let text = profile(name);
        for target in TARGETS {
            let config = Configuration::read(&text, target)
                .unwrap_or_else(|error| panic!("{name}.toml at {target}: {error}"));
            assert_eq!(
                config.name.as_deref(),
                Some(name),
                "{name}.toml states its own name so a profile copied between teams is identifiable"
            );
        }
    }
}

#[test]
fn refuse_any_loss_authorises_nothing() {
    // `doc/rfc/0007` section 5b.5's ninth finding: `refuse-any-loss` is today's behaviour stated
    // deliberately. It names no site, so it authorises no loss and departs from nothing — installing
    // it changes no pipeline, which is `doc/adr/0954`'s requirement.
    let text = profile("refuse-any-loss");
    let config = Configuration::read(&text, Target::Two(Level::B)).expect("it loads");
    assert_eq!(
        config.authorisations(Target::Two(Level::B)),
        pdf_transform::archive::Authorisations::default()
    );
    assert!(config.departures.is_empty());
}
