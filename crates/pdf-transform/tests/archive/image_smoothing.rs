//! An image dictionary's `/Interpolate true` is written false behind the loss `image-smoothing`
//! under both shipped profiles, at both parts (`doc/adr/2490`, `doc/adr/2498`).
//!
//! ISO 32000-2 §8.9.5.3 calls the entry "only a hint", which a processor may ignore; ISO 19005-2
//! section 6.2.8.1 and ISO 19005-4 section 6.2.7.1 require it false where an image states it. An
//! inline image's `/I` sits in a content stream, which this converter carries byte for byte
//! (`doc/adr/0816`), so that site keeps its refusal.

use super::*;
use pdf_transform::profiles::{Contribution, named};

const IMAGE: &str = "/Type /XObject /Subtype /Image /Width 1 /Height 1 \
                     /ColorSpace [/CalGray << /WhitePoint [0.9505 1.0 1.089] >>] \
                     /BitsPerComponent 8 /Interpolate true /Length 1";

const TARGETS: [Target; 2] = [Target::Two(Level::B), Target::Four(Flavour::Plain)];

/// A page painting one image that states `/Interpolate true`.
fn a_smoothed_image(target: Target) -> Vec<u8> {
    let base = match target {
        Target::Two(_) => Conforming::part_two(),
        Target::Four(_) => Conforming::default(),
    };
    Conforming {
        objects: vec![String::from_utf8_lossy(&stream(IMAGE, b"\x00")).into_owned()],
        resources: "/XObject << /Im0 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                    [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((String::new(), b"q 10 0 0 10 0 0 cm /Im0 Do Q".to_vec())),
        ..base
    }
    .build()
}

/// A page painting an inline image that states `/I true`.
fn an_inline_smoothed_image(target: Target) -> Vec<u8> {
    let base = match target {
        Target::Two(_) => Conforming::part_two(),
        Target::Four(_) => Conforming::default(),
    };
    Conforming {
        resources: "/ColorSpace << /CS0 [/CalGray << /WhitePoint [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((
            String::new(),
            b"q 10 0 0 10 0 0 cm BI /W 1 /H 1 /CS /CS0 /BPC 8 /I true ID \x00 EI Q".to_vec(),
        )),
        ..base
    }
    .build()
}

/// What a shipped profile contributes at `target`, as the command line and a window fold it in.
fn authorised_by(profile: &str, target: Target) -> Authorisations {
    Contribution::read(named(profile).expect("a shipped profile").text, target)
        .expect("a shipped profile reads")
        .authorised
}

#[test]
fn both_shipped_profiles_authorise_image_smoothing_at_both_parts() {
    for profile in ["as-if-printed", "as-shown"] {
        for target in TARGETS {
            assert!(
                authorised_by(profile, target).grants(Loss::ImageSmoothing),
                "{profile} at {target}"
            );
        }
    }
}

#[test]
fn a_smoothed_image_converts_under_each_profile_with_the_loss_named() {
    for profile in ["as-if-printed", "as-shown"] {
        for target in TARGETS {
            let source = a_smoothed_image(target);
            let (report, output) = convert(&source, target, authorised_by(profile, target));
            let output = written(&report, output);
            assert!(
                String::from_utf8_lossy(&output).contains("/Interpolate false"),
                "{profile} at {target}: the entry is written false"
            );
            assert_eq!(
                holds(&output, target).verdict(),
                Verdict::Conforms,
                "{profile} at {target}"
            );
            assert_eq!(
                decision(&report, "graphics/image-interpolation-is-off"),
                Decision::Authorised {
                    losses: Losses::one(Loss::ImageSmoothing),
                    rewrite: Rewrite::InterpolationOff,
                },
                "{profile} at {target}"
            );
            let packet = String::from_utf8(document_packet(&output)).expect("the packet is UTF-8");
            assert!(
                packet.contains("had /Interpolate set to false; loss image-smoothing: image 6 0 R"),
                "{profile} at {target}: xmpMM:History in the file names the loss and the image"
            );
        }
    }
}

#[test]
fn an_inline_images_interpolation_stays_refused_under_the_profiles() {
    for profile in ["as-if-printed", "as-shown"] {
        for target in TARGETS {
            let source = an_inline_smoothed_image(target);
            let (report, output) = convert(&source, target, authorised_by(profile, target));
            assert!(
                output.is_none(),
                "{profile} at {target}: nothing is written"
            );
            assert!(
                matches!(
                    decision(&report, "graphics/inline-image-interpolation-is-off"),
                    Decision::Refused(_)
                ),
                "{profile} at {target}"
            );
        }
    }
}
