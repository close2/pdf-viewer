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
use pdf_transform::archive::{Configuration, ReadFrom};

const TARGETS: [Target; 6] = [
    Target::Two(Level::B),
    Target::Two(Level::U),
    Target::Two(Level::A),
    Target::Four(Flavour::Plain),
    Target::Four(Flavour::F),
    Target::Four(Flavour::E),
];

const PROFILES: [&str; 8] = [
    "refuse-any-loss",
    "as-if-printed",
    // The owner's proposal beside it: the same yardstick asked of the window (ADR 1942).
    "as-shown",
    "only-metadata-loss",
    "keep-everything",
    // `doc/rfc/0007` section 3's own example, made real by the round that built `derive` and
    // `supply` — and the one shipped file that declares a `[tool.…]` block a site references, so
    // it is where the format's guardrails are exercised by something an operator would actually
    // install.
    "derive-attachments",
    // `doc/rfc/0014` section 7, `doc/questions/A706` items 3 and 4: information kept, else stop;
    // presentation discarded and stated — and the second, every loss stated.
    "no-data-loss",
    "state-every-loss",
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
fn no_shipped_profile_authorises_removing_a_transfer_function() {
    // `doc/adr/1964`: a graphics state's `/TR` replaces the device's initial transfer (ISO 32000-2
    // §8.4.1's Table 52) on a printer as on a screen, so both yardsticks say printing and the
    // window carried it, and the one corpus witness turns to a negative without it. The loss is
    // the operator's to authorise, by name. Both parts state the requirement, so the answer is the
    // same at every target.
    let loss = pdf_transform::archive::Loss::TransferFunction;
    for name in PROFILES {
        for target in TARGETS {
            let config = Configuration::read(&profile(name), target).expect("it loads");
            assert!(
                !config.authorisations(target).grants(loss),
                "{name} at {target}"
            );
        }
    }
    let mut authorised = pdf_transform::archive::Authorisations::default();
    authorised.authorise(loss);
    assert!(
        authorised.grants(loss),
        "and an operator's --authorise reaches it"
    );
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
    assert_eq!(derivations[0].tool.name, "to-pdf");
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

/// The shipped profiles, read off `doc/profiles/` rather than off [`PROFILES`]: every `.toml` there
/// that is a profile, which is every one but a file that says it is a departure instead.
fn profiles_on_disk() -> Vec<String> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/profiles");
    let mut names: Vec<String> = std::fs::read_dir(&directory)
        .unwrap_or_else(|error| panic!("{}: {error}", directory.display()))
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let name = path.file_stem()?.to_str()?.to_owned();
            let text = std::fs::read_to_string(&path).ok()?;
            (path.extension()? == "toml" && !text.contains("THIS IS NOT A PROFILE")).then_some(name)
        })
        .collect();
    names.sort();
    names
}

#[test]
fn every_profile_on_disk_is_held_here_and_named_by_the_command_line() {
    // ADR 1942: `--profile <name>` compiles the profiles in, so a profile added to `doc/profiles/`
    // and not to the program's list would be one an operator can read about and not choose.
    let mut held: Vec<String> = PROFILES.iter().map(|name| (*name).to_owned()).collect();
    held.sort();
    assert_eq!(
        profiles_on_disk(),
        held,
        "doc/profiles/ and this file's list agree"
    );
    for name in PROFILES {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_quorra-transform"))
            .args(["archive", "--remedy-sites", "--to", "2b", "--profile", name])
            .output()
            .expect("the program runs");
        let printed = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success() && printed.contains(&format!("for {name} at PDF/A-2b")),
            "--profile {name}: {}{}",
            printed,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// An operator's file whose packet site keeps the producer's own packet as a file: what the target
/// PDF/A-2b admits through its own qualified row, and what PDF/A-2u, the next target in
/// [`Target::ALL`]'s order, does not (`ATTACHING_NEEDS_A_FLAVOUR`, ISO 19005-2 section 6.8).
const FAILS_AT_TWO_U: &str = "[profile]
name = \"an-operator's-own\"

[site.\"metadata/xmp-packets-well-formed\"]
remedy = \"preserve\"
fresh-packet = true
original = \"attach\"

[site.\"metadata/xmp-packets-well-formed\".target.\"2b\"]
remedy = \"preserve\"
fresh-packet = true
original = \"page\"
";

/// The file written where the program can be pointed at it.
fn operator_file(name: &str, text: &str) -> std::path::PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::write(&path, text).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    path
}

/// `doc/rfc/0014` section 6, `doc/questions/A706` item 5: a run at PDF/A-2b, which the file reads
/// at, is refused because the file does not read at PDF/A-2u — and refused before any document,
/// which the input that does not exist shows: a run that had opened it would have said so.
#[test]
fn an_operators_file_failing_at_one_target_is_refused_before_any_document() {
    let config = operator_file("fails-at-2u.toml", FAILS_AT_TWO_U);
    assert!(Configuration::read(FAILS_AT_TWO_U, Target::Two(Level::B)).is_ok());
    assert!(Configuration::read(FAILS_AT_TWO_U, Target::Two(Level::U)).is_err());
    let absent = Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-such-document.pdf");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_quorra-transform"))
        .arg("archive")
        .arg(&absent)
        .args(["--to", "2b", "--in-process", "--config"])
        .arg(&config)
        .arg("-o")
        .arg(Path::new(env!("CARGO_TARGET_TMPDIR")).join("never-written.pdf"))
        .output()
        .expect("the program runs");
    let said = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{said}");
    assert!(
        said.contains("does not read at PDF/A-2u") && !said.contains("no-such-document"),
        "the file is refused at the target it fails at, and no document was reached: {said}"
    );
    // The arguments were right and the file was not, so the sentence is all that is printed.
    assert!(
        !said.contains("usage:"),
        "a configuration's refusal prints no usage text: {said}"
    );
    assert_eq!(output.status.code(), Some(1), "and keeps the usage status");
}

/// `--remedy-sites --config <file>` with no `--to`: the listings in [`Target::ALL`]'s order up to
/// the first the file does not read at, which ends the run non-zero; a shipped profile, which
/// reads at all six, prints all six and succeeds.
#[test]
fn remedy_sites_with_a_configuration_and_no_target_lists_the_six() {
    let config = operator_file("fails-at-2u-listing.toml", FAILS_AT_TWO_U);
    let failing = std::process::Command::new(env!("CARGO_BIN_EXE_quorra-transform"))
        .args(["archive", "--remedy-sites", "--config"])
        .arg(&config)
        .output()
        .expect("the program runs");
    let printed = String::from_utf8_lossy(&failing.stdout);
    let said = String::from_utf8_lossy(&failing.stderr);
    assert!(!failing.status.success(), "{printed}{said}");
    assert!(
        printed.contains("== PDF/A-2b ==")
            && printed.contains("== PDF/A-2u ==")
            && !printed.contains("== PDF/A-2a ==")
            && said.contains("does not read at PDF/A-2u"),
        "{printed}{said}"
    );
    let shipped = std::process::Command::new(env!("CARGO_BIN_EXE_quorra-transform"))
        .args([
            "archive",
            "--remedy-sites",
            "--profile",
            "derive-attachments",
        ])
        .output()
        .expect("the program runs");
    let printed = String::from_utf8_lossy(&shipped.stdout);
    assert!(shipped.status.success(), "{printed}");
    for target in TARGETS {
        assert!(
            printed.contains(&format!("== {target} ==")),
            "{target}: {printed}"
        );
    }
}

/// An operator's file stating one site over the shipped `as-if-printed`.
const OVER_AS_IF_PRINTED: &str = "[profile]
name = \"as-if-printed-keeping-unknown-annotations\"
base = \"as-if-printed\"

[site.\"annotations/subtype-defined-in-iso-32000-1\"]
remedy = \"stop\"
";

/// `doc/rfc/0014` section 6, `doc/questions/A706` item 5: `base` names a shipped profile, the
/// file's own site is the file's at every target, and every other site is the base's, marked so.
#[test]
fn a_base_answers_every_site_the_file_does_not() {
    let site = "annotations/subtype-defined-in-iso-32000-1";
    pdf_transform::profiles::read_at_every_target(OVER_AS_IF_PRINTED)
        .unwrap_or_else(|(target, error)| panic!("{target}: {error}"));
    for target in TARGETS {
        let mine = Configuration::read(OVER_AS_IF_PRINTED, target).expect("it loads");
        let base = Configuration::read(&profile("as-if-printed"), target).expect("it loads");
        assert_eq!(mine.base(), Some("as-if-printed"));
        assert_eq!(
            mine.name.as_deref(),
            Some("as-if-printed-keeping-unknown-annotations"),
            "a file never announces itself under its base's name"
        );
        let answered = mine.answered();
        assert_eq!(answered.len(), base.answered().len(), "{target}");
        for (answered_site, remedy, read_from) in answered {
            if answered_site == site {
                assert_eq!(remedy, pdf_transform::archive::RemedyKind::Stop);
                assert_eq!(read_from, ReadFrom::ThisFile);
            } else {
                assert_eq!(read_from, ReadFrom::Base("as-if-printed".to_owned()));
            }
        }
    }
    let unknown = OVER_AS_IF_PRINTED.replace("base = \"as-if-printed\"", "base = \"as-printed\"");
    let error = Configuration::read(&unknown, Target::Two(Level::B)).expect_err("no such base");
    assert!(error.to_string().contains("as-if-printed"), "{error}");
    let config = operator_file("over-as-if-printed.toml", OVER_AS_IF_PRINTED);
    let listed = std::process::Command::new(env!("CARGO_BIN_EXE_quorra-transform"))
        .args(["archive", "--remedy-sites", "--to", "2b", "--config"])
        .arg(&config)
        .output()
        .expect("the program runs");
    let printed = String::from_utf8_lossy(&listed.stdout);
    assert!(listed.status.success(), "{printed}");
    assert!(
        printed.contains(&format!("{site:?} is answered with `stop` (this file)"))
            && printed.contains("is answered with `discard` (from as-if-printed)"),
        "{printed}"
    );
}

#[test]
fn a_discarded_signature_row_authorises_the_signature_loss_at_every_target() {
    // `doc/adr/2542`: the conversion asks about a signature at every target, and ISO 19005-4 has
    // no requirement a part 4 profile could answer it at; the loss (the output carries no
    // signature) is one loss at both parts, so the 2b row's `discard` answers it wherever it is
    // asked — and a target-qualified `stop` at the same row is how a profile keeps it at part 4.
    let loss = pdf_transform::archive::Loss::SignatureAssertion;
    for name in ["as-if-printed", "as-shown"] {
        for target in TARGETS {
            let config = Configuration::read(&profile(name), target).expect("it loads");
            assert!(
                config.authorisations(target).grants(loss),
                "{name} at {target}"
            );
        }
    }
    for name in ["refuse-any-loss", "keep-everything"] {
        for target in TARGETS {
            let config = Configuration::read(&profile(name), target).expect("it loads");
            assert!(
                !config.authorisations(target).grants(loss),
                "{name} at {target}"
            );
        }
    }
    let kept_at_four = format!(
        "{}\n[site.\"file-structure/document-signature-states-no-digest\".target.\"4\"]\nremedy = \
         \"stop\"\n",
        profile("as-if-printed")
    );
    let four = Target::Four(Flavour::Plain);
    let config = Configuration::read(&kept_at_four, four).expect("it loads");
    assert!(!config.authorisations(four).grants(loss));
    let two = Target::Two(Level::B);
    let config = Configuration::read(&kept_at_four, two).expect("it loads");
    assert!(
        config.authorisations(two).grants(loss),
        "the qualifier decides part 4 alone"
    );
}

/// `doc/rfc/0014` section 6: a window offers the operator's own profiles from a directory the
/// host names, each held to the check a shipped one is held to — every target — and a file that
/// fails it, names a shipped profile or repeats an earlier file's name is not offered, with why.
#[test]
fn an_operators_directory_offers_the_files_that_read_at_every_target_and_names_the_rest() {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join("operators-profiles");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a directory");
    let write = |name: &str, text: &str| {
        std::fs::write(directory.join(name), text).expect("written");
    };
    write(
        "a-mine.toml",
        "[profile]\nname = \"mine\"\nsentence = \"as shown, from my own file\"\nbase = \"as-shown\"\n",
    );
    write("b-fails.toml", FAILS_AT_TWO_U);
    write(
        "c-shipped.toml",
        "[profile]\nname = \"as-shown\"\nbase = \"as-shown\"\n",
    );
    write("d-again.toml", "[profile]\nname = \"mine\"\n");
    write("e-nameless.toml", "[profile]\nsentence = \"no name\"\n");
    write("f-not-a-profile.txt", "ignored: not a .toml file");
    write(
        "g-long.toml",
        &" ".repeat(
            usize::try_from(pdf_transform::profiles::MOST_OPERATOR_PROFILE_BYTES + 1)
                .expect("fits"),
        ),
    );
    let (offered, refused) = pdf_transform::profiles::operators_profiles(&directory);
    assert_eq!(
        offered
            .iter()
            .map(|profile| profile.name.as_str())
            .collect::<Vec<_>>(),
        ["mine"]
    );
    assert_eq!(
        offered[0].sentence.as_deref(),
        Some("as shown, from my own file")
    );
    let why = |file: &str| {
        refused
            .iter()
            .find(|refused| refused.path.ends_with(file))
            .map_or_else(
                || panic!("{file} is named: {refused:?}"),
                |refused| refused.why.clone(),
            )
    };
    assert!(why("b-fails.toml").contains("does not read at PDF/A-2u"));
    assert!(why("c-shipped.toml").contains("shipped profile's name"));
    assert!(why("d-again.toml").contains("an earlier file"));
    assert!(why("e-nameless.toml").contains("states no [profile] name"));
    assert!(why("g-long.toml").contains("MOST_OPERATOR_PROFILE_BYTES"));
    assert_eq!(
        refused.len(),
        5,
        "the .txt file is not a profile: {refused:?}"
    );
}
