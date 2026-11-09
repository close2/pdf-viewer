//! Overprint mode 1 over a `DeviceCMYK` mark, where a part 4 file beside an RGB intent is given a
//! `/DefaultCMYK`, is written 0 as the named loss `overprint-mode` under a profile that accepts it
//! (`doc/adr/2749`).
//!
//! The expected values come from three clauses. ISO 32000-2 §8.6.5.6 makes a `DeviceCMYK` mark an
//! `ICCBased` one wherever a `DefaultCMYK` over an ICC profile is in force; ISO 19005-4 section
//! 6.2.4.2 forbids overprint mode 1 for an `ICCBased` CMYK mark painted with overprinting on; and
//! ISO 19005-4 section 6.2.4.3 leaves that default as the one licence for `DeviceCMYK` beside an
//! RGB destination profile. So the default is written only with the state's mode at 0, and §8.6.7,
//! which applies the mode to `DeviceCMYK` marks, makes that a press's change and the loss
//! `doc/adr/1978` names. §11.7.4.3's special blend mode is the one place this reader composites by
//! the mode, so a mark blended in four components keeps the refusal under every profile.

use super::{
    ArchivePlan, Conforming, Decision, Flavour, Level, Loss, Losses, Rewrite, SRGB, Target,
    Verdict, conversion, convert_with_plan, decision, document_packet, holds, not_built_sentence,
    outside_the_packet, plan_from,
};
use std::fmt::Write as _;

const AS_IF_PRINTED: &str = include_str!("../../../../doc/profiles/as-if-printed.toml");
const AS_SHOWN: &str = include_str!("../../../../doc/profiles/as-shown.toml");
const NO_DATA_LOSS: &str = include_str!("../../../../doc/profiles/no-data-loss.toml");

/// ISO 19005-4 section 6.2.4.3's `DeviceCMYK` row.
const ROW: &str = "graphics/device-cmyk-needs-a-default-a-blending-space-or-a-cmyk-output-intent";

/// ISO 19005-2 section 6.2.4.3's `DeviceCMYK` row.
const ROW_PART_TWO: &str = "graphics/device-cmyk-needs-a-default-or-a-cmyk-output-intent";

/// The history entry's sentence for the one state of [`source`], object 8.
const HISTORY: &str = "ISO 19005-4 section 6.2.4.2: 1 graphics state(s) had /OPM 1 written 0, and \
     the DeviceCMYK marks they governed with overprinting on paint their zero components; loss \
     overprint-mode: graphics state 8 0 R; on page(s) 1";

/// A plan converting to `target` under what the profile `text` states.
fn planned(text: &str, target: Target) -> ArchivePlan {
    let config =
        pdf_transform::archive::Configuration::read(text, target).expect("the profile reads");
    ArchivePlan {
        authorised: config.authorisations(target),
        ..plan_from(text, target)
    }
}

/// A file with its own sRGB intent (objects 6 and 7) whose page paints a black `DeviceCMYK`
/// rectangle with overprinting on under mode 1, set by the graphics state object 8; `page` is
/// added to the page dictionary. The file of `doc/adr/2749`'s nine, in one mark.
fn source(base: Conforming, page: &str) -> Vec<u8> {
    let mut hex = String::new();
    for byte in SRGB {
        let _ = write!(hex, "{byte:02X}");
    }
    hex.push('>');
    Conforming {
        catalog: "/OutputIntents [7 0 R]".to_owned(),
        page: page.to_owned(),
        resources: "/ExtGState << /G0 8 0 R >>".to_owned(),
        objects: vec![
            format!(
                "<< /N 3 /Filter /ASCIIHexDecode /Length {} >> stream\n{hex}\nendstream",
                hex.len()
            ),
            "<< /Type /OutputIntent /S /GTS_PDFA1 /OutputConditionIdentifier (sRGB) \
             /DestOutputProfile 6 0 R >>"
                .to_owned(),
            "<< /Type /ExtGState /OP true /op true /OPM 1 >>".to_owned(),
        ],
        contents: Some((String::new(), b"/G0 gs 0 0 0 1 k 10 10 50 50 re f".to_vec())),
        ..base
    }
    .build()
}

#[test]
fn the_default_is_written_with_the_mode_zeroed_as_the_named_loss_under_both_shipped_profiles() {
    // Section 6.2.4.3's default makes the mark ICCBased (§8.6.5.6), so section 6.2.4.2 holds only
    // with the state at mode 0. Both shipped profiles accept `overprint-mode`; `as-if-printed`'s
    // default is the shipped press, a loss of its own beside it (ADR 2518), and `as-shown`'s the
    // inks, which lose nothing else (ADR 2747).
    let target = Target::Four(Flavour::Plain);
    for (text, losses) in [
        (
            AS_IF_PRINTED,
            Losses::one(Loss::PressAssumed).with(Loss::OverprintMode),
        ),
        (AS_SHOWN, Losses::one(Loss::OverprintMode)),
    ] {
        let (report, output) =
            convert_with_plan(&source(Conforming::default(), ""), &planned(text, target));
        assert_eq!(
            decision(&report, ROW),
            Decision::Authorised {
                losses,
                rewrite: Rewrite::SuppliedDefaultCmyk,
            }
        );
        let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
        let held = holds(&output, target);
        assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
        let written = outside_the_packet(&output);
        assert!(
            written.contains("/OPM 0") && !written.contains("/OPM 1"),
            "{written}"
        );
        let json = conversion(&report).to_json().render();
        assert!(json.contains("overprint-mode"), "{json}");
        let packet = String::from_utf8(document_packet(&output)).expect("the packet is UTF-8");
        assert!(packet.contains(HISTORY), "{packet}");
    }
}

#[test]
fn a_profile_that_does_not_accept_the_loss_keeps_the_refusal() {
    // `no-data-loss` authorises no `overprint-mode`, so the mode stays and so does the reason no
    // default is written: the refusal names the mode rather than a loss nobody accepted.
    let target = Target::Four(Flavour::Plain);
    let (report, output) = convert_with_plan(
        &source(Conforming::default(), ""),
        &planned(NO_DATA_LOSS, target),
    );
    assert!(output.is_none(), "nothing is written");
    let sentence = not_built_sentence(&report, ROW);
    assert!(sentence.contains("overprint mode 1"), "{sentence}");
}

#[test]
fn a_mark_blended_in_four_components_keeps_the_refusal_under_as_if_printed() {
    // §11.7.4.3: in a page group blending in DeviceCMYK this reader composites the DeviceCMYK mark
    // by the mode, so writing it 0 would move the window (`doc/adr/1978`).
    let target = Target::Four(Flavour::Plain);
    let (report, output) = convert_with_plan(
        &source(
            Conforming::default(),
            "/Group << /S /Transparency /CS /DeviceCMYK >>",
        ),
        &planned(AS_IF_PRINTED, target),
    );
    assert!(output.is_none(), "nothing is written");
    let sentence = not_built_sentence(&report, ROW);
    assert!(sentence.contains("overprint mode 1"), "{sentence}");
}

#[test]
fn part_two_keeps_the_device_n_default_and_the_mode_beside_a_supplied_press() {
    // ISO 19005-2 section 6.2.4.3 admits a DeviceN-based DefaultCMYK, which leaves the mark a
    // DeviceCMYK one, so section 6.2.4.2 does not reach it and the mode costs nothing to keep:
    // a supplied press is not written over a state the conversion would have to change.
    let target = Target::Two(Level::B);
    let mut plan = planned(AS_IF_PRINTED, target);
    plan.profile = Some(std::sync::Arc::from(super::PRESS));
    let (report, output) = convert_with_plan(&source(Conforming::part_two(), ""), &plan);
    assert_eq!(
        decision(&report, ROW_PART_TWO).rewrite(),
        Some(Rewrite::DefaultCmyk),
        "{:?}",
        decision(&report, ROW_PART_TWO)
    );
    let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
    let held = holds(&output, target);
    assert_eq!(held.verdict(), Verdict::Conforms, "{}", held.render());
    assert!(outside_the_packet(&output).contains("/OPM 1"));
}
