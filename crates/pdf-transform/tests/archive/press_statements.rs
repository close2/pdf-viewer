//! Two press statements `as-if-printed` takes out as `as-shown` does, each a loss named in the report
//! and in the file's own `xmpMM:History` (R17 of `doc/adr/2490`, `doc/adr/2693`).
//!
//! ISO 19005-2 and ISO 19005-4 section 6.2.5 forbid a halftone's `HalftoneName`, and section 6.2.4.2
//! of each forbids overprint mode 1 where an `ICCBased` CMYK space is used with overprinting on. The
//! expected values come from ISO 32000-2: §10.6.5.1 makes the name a pointer a press with a halftone
//! of that name may follow, and the dictionary's own entries define the screen without it (so the
//! `/SpotFunction` stays); §8.6.7 applies mode 1 to `DeviceCMYK` marks alone, so a file whose
//! overprinted marks are all `ICCBased` loses nothing and records nothing (`doc/adr/1978`); and
//! §11.7.4.3's special blend mode is the one place this reader composites by the mode, which stays
//! refused under every profile.

use super::{
    ArchivePlan, Because, Conforming, Decision, Flavour, Level, Loss, Losses, Rewrite, Target,
    Verdict, conversion, convert_with_plan, decision, document_packet, holds, plan_from,
};

const AS_IF_PRINTED: &str = include_str!("../../../../doc/profiles/as-if-printed.toml");
const AS_SHOWN: &str = include_str!("../../../../doc/profiles/as-shown.toml");
const NO_DATA_LOSS: &str = include_str!("../../../../doc/profiles/no-data-loss.toml");

const HALFTONE_NAME: &str = "graphics/no-halftone-name";
const OVERPRINT_MODE: &str = "graphics/no-overprint-mode-one-under-icc-cmyk";

const TARGETS: [Target; 6] = [
    Target::Two(Level::B),
    Target::Two(Level::U),
    Target::Two(Level::A),
    Target::Four(Flavour::Plain),
    Target::Four(Flavour::F),
    Target::Four(Flavour::E),
];

/// A plan converting to `target` under what the profile `text` states.
fn planned(text: &str, target: Target) -> ArchivePlan {
    let config =
        pdf_transform::archive::Configuration::read(text, target).expect("the profile reads");
    ArchivePlan {
        authorised: config.authorisations(target),
        ..plan_from(text, target)
    }
}

/// The fixture a conversion to `target` starts from: a PDF 1.7 file for part 2, a 2.0 one for 4.
fn base(target: Target) -> Conforming {
    match target {
        Target::Two(_) => Conforming::part_two(),
        Target::Four(_) => Conforming::default(),
    }
}

/// A page filling one rectangle under a graphics state whose halftone is Acrobat Distiller's type 1
/// halftone naming the device's `Default`, with its own frequency, angle and spot function.
fn a_named_halftone(target: Target) -> Vec<u8> {
    Conforming {
        resources: "/ExtGState << /G0 6 0 R >>".to_owned(),
        objects: vec![
            "<< /Type /ExtGState /HT 7 0 R >>".to_owned(),
            super::DISTILLER_HALFTONE.to_owned(),
        ],
        contents: Some((String::new(), b"/G0 gs 0 0 1 rg 10 10 50 50 re f".to_vec())),
        ..base(target)
    }
    .build()
}

#[test]
fn as_if_printed_authorises_both_press_losses_as_as_shown_does_at_every_target() {
    for target in TARGETS {
        for (name, text) in [("as-if-printed", AS_IF_PRINTED), ("as-shown", AS_SHOWN)] {
            let authorised = planned(text, target).authorised;
            for loss in [Loss::HalftoneName, Loss::OverprintMode] {
                assert!(authorised.grants(loss), "{name} at {target}: {loss:?}");
            }
        }
        // `no-data-loss` takes no loss a statement cannot carry: the halftone's name is stated on
        // the page it appends, and the overprint mode, which that page cannot carry, stays a stop.
        let authorised = planned(NO_DATA_LOSS, target).authorised;
        assert!(!authorised.grants(Loss::OverprintMode), "at {target}");
    }
}

#[test]
fn a_halftone_name_goes_under_as_if_printed_named_in_the_report_and_the_history() {
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let source = a_named_halftone(target);
        let mut histories = Vec::new();
        for text in [AS_IF_PRINTED, AS_SHOWN] {
            let (report, output) = convert_with_plan(&source, &planned(text, target));
            assert_eq!(
                decision(&report, HALFTONE_NAME),
                Decision::Authorised {
                    losses: Losses::one(Loss::HalftoneName),
                    rewrite: Rewrite::HalftoneNameRemoved,
                },
                "at {target}"
            );
            let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
            let held = holds(&output, target);
            assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
            let text = String::from_utf8_lossy(&output);
            assert!(
                !super::outside_the_packet(&output).contains("HalftoneName"),
                "at {target}: no dictionary states the key"
            );
            assert!(
                text.contains("/SpotFunction /Round"),
                "at {target}: the screen the dictionary defines stays"
            );
            let json = conversion(&report).to_json().render();
            assert!(json.contains("halftone-name"), "{json}");
            let packet = String::from_utf8(document_packet(&output)).expect("the packet is UTF-8");
            let entry = packet
                .split("stEvt:parameters")
                .find(|part| part.contains("/HalftoneName"))
                .unwrap_or_else(|| panic!("at {target}: no history entry\n{packet}"));
            assert!(
                entry.contains(
                    "1 halftone(s) had /HalftoneName removed, every parameter they define kept; \
                     loss halftone-name: halftone 7 0 R named (Default)"
                ),
                "at {target}: {entry}"
            );
            histories.push(entry.to_owned());
        }
        assert_eq!(
            histories[0], histories[1],
            "one mechanism: both profiles write the same entry"
        );
    }
}

#[test]
fn an_overprint_mode_over_a_device_cmyk_mark_goes_under_as_if_printed_named_in_both_places() {
    let target = Target::Two(Level::B);
    let source = super::an_overprint_mode("0 0 0 1 k 20 20 5 5 re f", "");
    let mut histories = Vec::new();
    for text in [AS_IF_PRINTED, AS_SHOWN] {
        let (report, output) = convert_with_plan(&source, &planned(text, target));
        assert_eq!(
            decision(&report, OVERPRINT_MODE),
            Decision::Authorised {
                losses: Losses::one(Loss::OverprintMode),
                rewrite: Rewrite::OverprintModeZeroed,
            }
        );
        let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
        let held = holds(&output, target);
        assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
        let written = super::outside_the_packet(&output);
        assert!(written.contains("/OPM 0") && !written.contains("/OPM 1"));
        let json = conversion(&report).to_json().render();
        assert!(json.contains("overprint-mode"), "{json}");
        let packet = String::from_utf8(document_packet(&output)).expect("the packet is UTF-8");
        let entry = packet
            .split("stEvt:parameters")
            .find(|part| part.contains("/OPM 1"))
            .unwrap_or_else(|| panic!("no history entry\n{packet}"));
        assert!(
            entry.contains(
                "1 graphics state(s) had /OPM 1 written 0, and the DeviceCMYK marks they governed \
                 with overprinting on paint their zero components; loss overprint-mode: graphics \
                 state 6 0 R; on page(s) 1"
            ),
            "{entry}"
        );
        histories.push(entry.to_owned());
    }
    assert_eq!(histories[0], histories[1], "one mechanism");
}

#[test]
fn an_overprint_mode_governing_only_icc_marks_is_stated_and_recorded_nowhere() {
    // §8.6.7: with no DeviceCMYK mark under the state, mode 0 moves nothing the clause defines, so
    // the restatement is no loss and the file's history has nothing to say of it.
    let target = Target::Two(Level::B);
    let (report, output) = convert_with_plan(
        &super::an_overprint_mode("", ""),
        &planned(AS_IF_PRINTED, target),
    );
    assert!(matches!(
        decision(&report, OVERPRINT_MODE),
        Decision::Stated {
            rewrite: Rewrite::OverprintModeRestated,
            ..
        }
    ));
    let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
    let packet = String::from_utf8(document_packet(&output)).expect("the packet is UTF-8");
    assert!(!packet.contains("/OPM"), "{packet}");
}

#[test]
fn no_data_loss_keeps_refusing_the_overprint_mode_by_name() {
    let target = Target::Two(Level::B);
    let source = super::an_overprint_mode("0 0 0 1 k 20 20 5 5 re f", "");
    let (report, output) = convert_with_plan(&source, &planned(NO_DATA_LOSS, target));
    assert!(matches!(
        decision(&report, OVERPRINT_MODE),
        Decision::Unauthorised { losses, .. } if losses == Losses::one(Loss::OverprintMode)
    ));
    assert!(output.is_none());
}

#[test]
fn an_overprint_mode_the_window_composites_by_stays_refused_under_as_if_printed() {
    // §11.7.4.3: inside a page group blending in DeviceCMYK this reader composites the DeviceCMYK
    // mark by the mode, so the loss would be one the window showed (`doc/adr/1978`).
    let target = Target::Two(Level::B);
    let source = super::an_overprint_mode(
        "0 0 0 1 k 20 20 5 5 re f",
        "/Group << /S /Transparency /CS /DeviceCMYK >>",
    );
    let (report, output) = convert_with_plan(&source, &planned(AS_IF_PRINTED, target));
    assert!(matches!(
        decision(&report, OVERPRINT_MODE),
        Decision::Refused(Because::NotBuiltYet(why)) if why.contains("four components")
    ));
    assert!(output.is_none());
}
