//! The operator's CMYK press, named once in a configuration's `[profile] press` and used by every
//! conversion that reads it (`doc/questions/A784`, `doc/adr/2704`, `doc/adr/2705`).
//!
//! The expected values come from the rules those records state, not from any renderer: the press
//! stands in for the CMYK profile this program ships and for nothing else, so it is written only
//! where `press-assumed` is taken (ADRs 2518, 2701); `--output-intent-profile` outranks it as it
//! outranks the shipped press (ADR 2130); and it is held to what ISO 19005-2 and ISO 19005-4
//! section 6.2.3 ask of a destination profile and section 6.2.4.2 asks of an `ICCBased` stream's
//! profile, narrowed to an output profile over CMYK. The press the tests configure is the profile
//! of this renderer's assumed inks, the one licence-clean CMYK profile the tree holds besides the
//! shipped one, so the two can be told apart byte for byte.

use super::*;

/// The configured press of these tests (`data/icc/PROVENANCE.md`'s last section).
const INKS: &[u8] = include_bytes!("../../../../data/icc/quorra-assumed-inks.icc");

/// The shipped sRGB profile, a monitor profile over RGB: an ICC profile that is no press.
const SRGB: &[u8] = include_bytes!("../../../../data/icc/sRGB2014.icc");

/// The row part 4 fails for a `DeviceCMYK` use beside the file's own RGB intent.
const PART_FOUR_ROW: &str =
    "graphics/device-cmyk-needs-a-default-a-blending-space-or-a-cmyk-output-intent";

/// What a configuration based on `as-if-printed` that names `inks.icc` says.
const BASED_ON_AS_IF_PRINTED: &str = "[profile]\nbase = \"as-if-printed\"\npress = \"inks.icc\"\n";

/// The loss the configured press stands behind, and nothing else.
fn press_assumed() -> Authorisations {
    let mut authorised = Authorisations::default();
    authorised.authorise(Loss::PressAssumed);
    authorised
}

/// A part 2 file with no output intent that paints in `DeviceCMYK` alone, so the one intent a
/// conversion adds is a CMYK one (`doc/adr/1153`).
fn cmyk_without_an_intent() -> Vec<u8> {
    Conforming {
        contents: Some(paints_in_device_cmyk()),
        ..Conforming::part_two()
    }
    .build()
}

/// A part 4 file with its own RGB intent that paints one black in `DeviceCMYK`.
fn cmyk_beside_an_rgb_intent() -> Vec<u8> {
    with_its_own_rgb_intent(Conforming::default(), "", "0 0 0 1 k 0 0 10 10 re f")
}

/// Converts `source` to `target` with `authorised`, the flag's profile and the configured press.
fn converted(
    source: &[u8],
    target: Target,
    authorised: Authorisations,
    profile: Option<&[u8]>,
    press: Option<&[u8]>,
) -> (Report, Option<Vec<u8>>) {
    let sinks = MemorySinks::new();
    let mut plan = plan_from("", target);
    plan.authorised = authorised;
    plan.profile = profile.map(std::sync::Arc::from);
    plan.press = press.map(std::sync::Arc::from);
    let report = apply(
        &Plan::Archive(plan),
        &[Source::new(source.to_vec())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the conversion applies");
    (report, sinks.into_outputs().pop().map(|(_, bytes)| bytes))
}

/// The decoded destination profile of the output's first output intent.
fn destination_profile(output: &[u8]) -> Vec<u8> {
    let document = Document::open_with_limits(output.to_vec(), Limits::DEFAULT).expect("it opens");
    let catalog = document.catalog().expect("a catalog");
    let intents = document.get_key(&catalog, "OutputIntents");
    let first = intents
        .as_array()
        .and_then(|intents| intents.first())
        .map(|intent| document.resolve(intent))
        .and_then(|intent| intent.as_dict().cloned())
        .expect("an output intent");
    let profile = document.get_key(&first, "DestOutputProfile");
    let stream = profile.as_stream().expect("a profile stream");
    document
        .decoded_stream_data(stream)
        .expect("it decodes")
        .to_vec()
}

#[test]
fn as_if_printed_states_the_configured_press_as_the_intent_and_the_shipped_one_without_it() {
    let target = Target::Two(Level::B);
    let row = "graphics/device-cmyk-needs-a-default-or-a-cmyk-output-intent";
    // Without the setting: the shipped press, as ADR 2701 has it.
    let (report, output) = converted(
        &cmyk_without_an_intent(),
        target,
        press_assumed(),
        None,
        None,
    );
    let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
    assert_eq!(destination_profile(&output), PRESS);
    let named = conversion(&report).profile.as_ref().expect("a profile");
    assert_eq!(named.source, pdf_transform::archive::ProfileSource::Shipped);
    // With it: the operator's press, byte for byte, named as theirs with its own copyright.
    let (report, output) = converted(
        &cmyk_without_an_intent(),
        target,
        press_assumed(),
        None,
        Some(INKS),
    );
    let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    assert_eq!(destination_profile(&output), INKS);
    let named = conversion(&report).profile.as_ref().expect("a profile");
    assert_eq!(
        named.source,
        pdf_transform::archive::ProfileSource::Configured
    );
    assert_eq!(named.describes.as_deref(), Some(ASSUMED_INKS_PROFILE));
    let text = conversion(&report).render();
    assert!(
        text.contains("the press your configuration names") && text.contains("copyright tag"),
        "{text}"
    );
    assert!(
        conversion(&report)
            .to_json()
            .render()
            .contains("\"configured\""),
        "the report's machine-readable form names the source"
    );
    let Decision::Stated { reinterprets, .. } = decision(&report, row) else {
        panic!("the intent, stated: {:?}", decision(&report, row));
    };
    assert!(
        reinterprets.contains("your configuration names") && !reinterprets.contains("ships"),
        "{reinterprets}"
    );
}

#[test]
fn as_if_printed_writes_the_configured_press_as_part_fours_default_cmyk_and_names_it() {
    let target = Target::Four(Flavour::Plain);
    let (report, output) = converted(
        &cmyk_beside_an_rgb_intent(),
        target,
        press_assumed(),
        None,
        Some(INKS),
    );
    let Decision::Authorised { losses, rewrite } = decision(&report, PART_FOUR_ROW) else {
        panic!(
            "the loss, authorised: {:?}",
            decision(&report, PART_FOUR_ROW)
        );
    };
    assert_eq!(rewrite, Rewrite::SuppliedDefaultCmyk);
    assert!(losses.iter().eq([Loss::PressAssumed]));
    let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let document = Document::open_with_limits(output.clone(), Limits::DEFAULT).expect("it opens");
    let space = default_cmyk(&document);
    let stream = document.resolve(space.get(1).expect("a profile stream"));
    let stream = stream.as_stream().expect("a stream");
    assert_eq!(document.decoded_stream_data(stream).as_deref(), Some(INKS));
    let named = conversion(&report)
        .press_assumed
        .as_ref()
        .expect("the report names the press");
    assert_eq!(
        named.source,
        pdf_transform::archive::ProfileSource::Configured
    );
    assert!(named.copyright.is_some(), "its own cprt is carried");
    let text = conversion(&report).render();
    assert!(
        text.contains("the press your configuration names") && text.contains("its copyright tag"),
        "{text}"
    );
    let packet = String::from_utf8(document_packet(&output)).expect("the packet is UTF-8");
    assert!(
        packet.contains("names in their configuration") && packet.contains("press-assumed"),
        "{packet}"
    );
    assert!(!packet.contains("GRACoL"), "{packet}");
}

#[test]
fn the_flag_outranks_a_configured_press_on_both_routes() {
    let (report, output) = converted(
        &cmyk_without_an_intent(),
        Target::Two(Level::B),
        press_assumed(),
        Some(PRESS),
        Some(INKS),
    );
    let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
    assert_eq!(destination_profile(&output), PRESS);
    let named = conversion(&report).profile.as_ref().expect("a profile");
    assert_eq!(
        named.source,
        pdf_transform::archive::ProfileSource::Supplied
    );

    let (report, _) = converted(
        &cmyk_beside_an_rgb_intent(),
        Target::Four(Flavour::Plain),
        press_assumed(),
        Some(PRESS),
        Some(INKS),
    );
    let Decision::Stated { reinterprets, .. } = decision(&report, PART_FOUR_ROW) else {
        panic!(
            "the operator's statement: {:?}",
            decision(&report, PART_FOUR_ROW)
        );
    };
    assert!(reinterprets.contains("you supplied"), "{reinterprets}");
    assert!(conversion(&report).press_assumed.is_none());
}

#[test]
fn a_configured_press_does_not_replace_the_assumed_inks() {
    // A profile that takes no assumed press states the inks this renderer drew the source through
    // (ADR 2701), whatever press is configured: the press stands in for the shipped one only.
    let (report, output) = converted(
        &cmyk_without_an_intent(),
        Target::Two(Level::B),
        Authorisations::default(),
        None,
        Some(PRESS),
    );
    let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
    assert_eq!(destination_profile(&output), INKS);
    let named = conversion(&report).profile.as_ref().expect("a profile");
    assert_eq!(named.source, pdf_transform::archive::ProfileSource::Shipped);
    assert_eq!(named.describes.as_deref(), Some(ASSUMED_INKS_PROFILE));
}

#[test]
fn a_configuration_names_its_press_once_for_every_target() {
    for target in Target::ALL {
        let read = pdf_transform::profiles::Contribution::read(BASED_ON_AS_IF_PRINTED, target)
            .expect("it reads");
        assert_eq!(read.press.as_deref(), Some("inks.icc"), "at {target}");
        assert!(read.authorised.grants(Loss::PressAssumed), "at {target}");
    }
    // No shipped profile names a press: one compiled in has no directory to name it from.
    for shipped in pdf_transform::profiles::SHIPPED {
        for target in Target::ALL {
            let read = pdf_transform::profiles::Contribution::read(shipped.text, target)
                .expect("a shipped profile reads");
            assert!(read.press.is_none(), "{} at {target}", shipped.name);
        }
    }
}

#[test]
fn a_press_key_the_configuration_cannot_use_is_refused_by_its_sentence() {
    let target = Target::Four(Flavour::Plain);
    let refused = |text: &str| {
        pdf_transform::profiles::Contribution::read(text, target)
            .expect_err("refused")
            .to_string()
    };
    let never = refused("[profile]\nbase = \"as-shown\"\npress = \"inks.icc\"\n");
    assert!(
        never.contains("authorises the loss press-assumed at no site")
            && never.contains("no conversion would use it"),
        "{never}"
    );
    let empty = refused("[profile]\nbase = \"as-if-printed\"\npress = \"\"\n");
    assert!(
        empty.contains("press") && empty.contains("an empty string"),
        "{empty}"
    );
    let number = refused("[profile]\nbase = \"as-if-printed\"\npress = true\n");
    assert!(
        number.contains("the path of a CMYK output ICC profile"),
        "{number}"
    );
}

/// `bytes` with the header's device class or version overwritten (ICC.1:1998-09 section 6.1).
fn restated(bytes: &[u8], at: usize, with: &[u8]) -> Vec<u8> {
    let mut changed = bytes.to_vec();
    for (offset, byte) in with.iter().enumerate() {
        // The offsets are a header's (ICC.1:1998-09 section 6.1), inside any profile's 128 bytes.
        changed[at.checked_add(offset).expect("inside the header")] = *byte;
    }
    changed
}

#[test]
fn a_file_that_is_no_press_the_target_admits_is_refused_by_its_sentence() {
    use pdf_transform::archive::configured_press;
    let two = Target::Two(Level::B);
    let four = Target::Four(Flavour::Plain);
    let sentence = |bytes: &[u8], target| {
        configured_press(bytes, target)
            .expect_err("refused")
            .to_string()
    };
    assert_eq!(
        sentence(b"not a profile", two),
        "this file is not an ICC profile"
    );
    assert_eq!(
        sentence(SRGB, two),
        "a press is an output (prtr) profile over CMYK, and this one is a mntr profile over RGB"
    );
    assert_eq!(
        sentence(&restated(INKS, 12, b"mntr"), four),
        "a press is an output (prtr) profile over CMYK, and this one is a mntr profile over CMYK"
    );
    // Version 4.2.0 is not among part 2's four texts and is before ICC.1:2010 for part 4.
    let four_two = restated(INKS, 8, &[4, 0x20]);
    assert!(
        sentence(&four_two, two).starts_with("ISO 19005-2 section 6.2.4.2 admits"),
        "{}",
        sentence(&four_two, two)
    );
    assert!(sentence(&four_two, two).ends_with("states version 4.2.0"));
    assert!(configured_press(&four_two, four).is_ok());
    let iccmax = restated(INKS, 8, &[5, 0]);
    assert!(
        sentence(&iccmax, four).contains("ICC.1:2010")
            && sentence(&iccmax, four).ends_with("states version 5.0.0"),
        "{}",
        sentence(&iccmax, four)
    );
}

/// The command line, run in `at`, with what it printed on each stream.
fn run_in(at: &std::path::Path, arguments: &[&str]) -> (bool, String, String) {
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_quorra-transform"))
        .current_dir(at)
        .args(arguments)
        .output()
        .expect("the program runs");
    (
        run.status.success(),
        String::from_utf8_lossy(&run.stdout).into_owned(),
        String::from_utf8_lossy(&run.stderr).into_owned(),
    )
}

/// A scratch directory holding `in.pdf` and `company/press.toml` naming `company/<press>`.
fn a_company(name: &str, press: &str, bytes: &[u8]) -> Scratch {
    let scratch = Scratch::make(name);
    std::fs::write(scratch.0.join("in.pdf"), cmyk_without_an_intent()).expect("the source");
    let company = scratch.0.join("company");
    std::fs::create_dir_all(&company).expect("the company's directory");
    std::fs::write(
        company.join("press.toml"),
        format!("[profile]\nbase = \"as-if-printed\"\npress = \"{press}\"\n"),
    )
    .expect("the configuration");
    std::fs::write(company.join(press), bytes).expect("the press");
    std::fs::write(scratch.0.join("gracol.icc"), PRESS).expect("the flag's profile");
    scratch
}

#[test]
fn the_command_line_reads_the_press_beside_its_configuration_and_the_flag_outranks_it() {
    let scratch = a_company("press-beside", "inks.icc", INKS);
    // Run from the scratch directory, so `inks.icc` resolves only against the configuration's own.
    let (ok, said, warned) = run_in(
        &scratch.0,
        &[
            "archive",
            "in.pdf",
            "--to",
            "2b",
            "--config",
            "company/press.toml",
            "-o",
            "out.pdf",
        ],
    );
    assert!(ok, "{said}\n{warned}");
    assert!(
        warned.contains("names the press company/inks.icc") && warned.contains("copyright tag"),
        "{warned}"
    );
    assert!(
        warned.contains("the press your configuration names"),
        "{said}\n{warned}"
    );
    let output = std::fs::read(scratch.0.join("out.pdf")).expect("written");
    assert_eq!(destination_profile(&output), INKS);

    let (ok, said, warned) = run_in(
        &scratch.0,
        &[
            "archive",
            "in.pdf",
            "--to",
            "2b",
            "--config",
            "company/press.toml",
            "--output-intent-profile",
            "gracol.icc",
            "-o",
            "flag.pdf",
        ],
    );
    assert!(ok, "{said}\n{warned}");
    let output = std::fs::read(scratch.0.join("flag.pdf")).expect("written");
    assert_eq!(destination_profile(&output), PRESS);
}

#[test]
fn the_command_line_refuses_a_press_it_cannot_use_naming_the_file() {
    let scratch = a_company("press-refused", "srgb.icc", SRGB);
    let (ok, _, warned) = run_in(
        &scratch.0,
        &[
            "archive",
            "in.pdf",
            "--to",
            "2b",
            "--config",
            "company/press.toml",
            "-o",
            "out.pdf",
        ],
    );
    assert!(!ok);
    assert!(
        warned.contains("[profile] press names company/srgb.icc")
            && warned.contains("this one is a mntr profile over RGB"),
        "{warned}"
    );
    std::fs::write(
        scratch.0.join("company/press.toml"),
        "[profile]\nbase = \"as-if-printed\"\npress = \"missing.icc\"\n",
    )
    .expect("the configuration");
    let (ok, _, warned) = run_in(
        &scratch.0,
        &[
            "archive",
            "in.pdf",
            "--to",
            "2b",
            "--config",
            "company/press.toml",
            "-o",
            "out.pdf",
        ],
    );
    assert!(!ok);
    assert!(
        warned.contains("[profile] press names company/missing.icc, which cannot be read"),
        "{warned}"
    );
    assert!(!scratch.0.join("out.pdf").exists());
}
