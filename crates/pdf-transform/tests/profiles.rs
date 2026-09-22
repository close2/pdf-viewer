//! The shipped profiles, held to the configuration format they are written in.
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

const PROFILES: [&str; 5] = [
    "refuse-any-loss",
    "as-if-printed",
    "only-metadata-loss",
    "keep-everything",
    // `doc/rfc/0007` section 3's own example, made real by the round that built `derive` and
    // `supply` — and the one shipped file that declares a `[tool.…]` block a site references, so
    // it is where the format's guardrails are exercised by something an operator would actually
    // install.
    "derive-attachments",
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

#[test]
fn every_profile_declaring_a_tool_carries_the_warning_the_owner_asked_for() {
    // **`doc/questions/A56`, as a property of the shipped files rather than as a habit.** Of the
    // options put, the owner chose *no offer in the first version, warn at the configuration
    // site* — so the sentence lives where an operator declares a program, and a profile that grew
    // a `[tool.…]` block without it would be one whose reader is never told. The other place the
    // warning is printed is `archive --remedy-sites`, for a site that takes one.
    for name in PROFILES {
        let text = profile(name);
        if !text.contains("[tool.") {
            continue;
        }
        assert!(
            text.contains(pdf_transform::archive::UNTRUSTED_INPUT_WARNING),
            "{name}.toml declares a tool and does not say: {}",
            pdf_transform::archive::UNTRUSTED_INPUT_WARNING
        );
    }
}

#[test]
fn the_derive_profile_names_a_site_and_a_tool_together() {
    // `doc/questions/A55`'s guardrail, read off a file an operator would install: a `derive` row
    // that named only the site would not load at all — `ConfigError::DeriveWithoutTool` — so the
    // shipped example cannot drift into being one.
    let text = profile("derive-attachments");
    let target = Target::Four(Flavour::Plain);
    let config = Configuration::read(&text, target).expect("it loads");
    let derivations = config.derivations(target);
    assert_eq!(
        derivations.len(),
        1,
        "one of the two spellings binds part 4"
    );
    assert_eq!(derivations[0].tool.name, "office-to-pdf");
    assert_eq!(derivations[0].tool.expects, "application/pdf");
    // And the `supply` it ships is built too, so the file promises nothing it cannot keep.
    assert_eq!(config.supplies(Target::Four(Flavour::F)).len(), 1);
}

#[test]
fn keep_everything_offers_an_attachment_only_where_the_target_holds_one() {
    // ISO 19005-2 section 6.8 and ISO 19005-4 section 6.9 admit an embedded file only where it is
    // itself PDF/A, and Annexes A and B of ISO 19005-4 lift that for 4f and 4e. So at the four
    // other targets this profile's order — keep it, else stop — has nothing to keep these in, and
    // the rows say `stop` rather than naming an attachment nothing can carry out.
    const ATTACH_ONLY: [&str; 6] = [
        "annotations/appearance-dictionary-holds-only-normal",
        "forms/no-action-on-widget-or-field",
        "forms/no-xfa-key",
        "graphics/no-transfer-function-in-a-graphics-state",
        "graphics/second-transfer-function-is-default",
        "graphics/halftone-transfer-function-only-where-required",
    ];
    let text = profile("keep-everything");
    for target in [
        Target::Two(Level::B),
        Target::Two(Level::U),
        Target::Two(Level::A),
        Target::Four(Flavour::Plain),
    ] {
        let config = Configuration::read(&text, target).expect("it loads");
        let unbuilt = config.unbuilt(target);
        for site in ATTACH_ONLY {
            assert!(
                !unbuilt.iter().any(|row| row.site == site),
                "{site} at {target} names an attachment the target cannot hold"
            );
        }
    }
    // At 4f the intent is still stated, whether or not its mechanism is built, so the row is
    // not silently dropped where it can mean something.
    let four_f = Target::Four(Flavour::F);
    let config = Configuration::read(&text, four_f).expect("it loads");
    let unbuilt = config.unbuilt(four_f);
    assert!(
        unbuilt
            .iter()
            .any(|row| row.site == "graphics/no-transfer-function-in-a-graphics-state"),
        "the 4f row still asks for the attachment"
    );
}
