//! A stream whose filter no reader of the standard decodes is written as this reader drew it,
//! absent, behind the loss `undecodable-stream` (ADR 2490's R07, `doc/adr/2689`).
//!
//! ISO 32000-2 §7.4 makes the filters a stream states the only route to its data
//! (`doc/adr/2679`), and ISO 19005-2 section 6.1.7.2 and ISO 19005-4 section 6.1.6.2 admit none
//! but the standard's. Both shipped profiles and `state-every-loss` take the loss; `no-data-loss`
//! refuses by its name.

use super::*;
use pdf_syntax::object::{Name, Object};
use pdf_transform::archive::UndecodableRole;
use pdf_transform::profiles::{Contribution, named};

const ROW: &str = "file-structure/stream-filters-are-standard";

const TARGETS: [Target; 2] = [Target::Two(Level::B), Target::Four(Flavour::Plain)];

/// What a profile contributes at `target`, as the command line folds it in.
fn authorised_by(profile: &str, target: Target) -> Authorisations {
    Contribution::read(named(profile).expect("a shipped profile").text, target)
        .expect("a shipped profile reads")
        .authorised
}

fn base(target: Target) -> Conforming {
    match target {
        Target::Two(_) => Conforming::part_two(),
        Target::Four(_) => Conforming::default(),
    }
}

/// A page painting object 6, an `XObject` whose dictionary is `dict` and whose filter is
/// `/XXXDecode`, the name the PDF Association's `UnknownFilter` files use.
fn painting(target: Target, dict: &str) -> Vec<u8> {
    let dict = format!("{dict} /Filter /XXXDecode /Length 3");
    Conforming {
        objects: vec![String::from_utf8_lossy(&stream(&dict, b"xyz")).into_owned()],
        resources: "/XObject << /X0 6 0 R >>".to_owned(),
        contents: Some((String::new(), b"q 10 0 0 10 0 0 cm /X0 Do Q".to_vec())),
        ..base(target)
    }
    .build()
}

const IMAGE: &str = "/Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray \
                     /BitsPerComponent 8";
const FORM: &str = "/Type /XObject /Subtype /Form /BBox [0 0 1 1]";

/// Object 6 of `output`, which the writer keeps at its number for a stream it writes absent.
fn object_six(output: &[u8]) -> Object {
    let document =
        Document::open_with_limits(output.to_vec(), Limits::DEFAULT).expect("the output opens");
    let pages = pdf_model::Pages::new(&document);
    let page = pages.get(0).expect("one page");
    let resources = document.get_key(&page.dict, "Resources");
    let xobjects = document.get_key(resources.as_dict().expect("resources"), "XObject");
    document.get_key(xobjects.as_dict().expect("an XObject table"), "X0")
}

#[test]
fn the_shipped_profiles_and_state_every_loss_take_the_loss_and_no_data_loss_does_not() {
    for target in TARGETS {
        for profile in ["as-if-printed", "as-shown", "state-every-loss"] {
            assert!(
                authorised_by(profile, target).grants(Loss::UndecodableStream),
                "{profile} at {target}"
            );
        }
        assert!(
            !authorised_by("no-data-loss", target).grants(Loss::UndecodableStream),
            "no-data-loss at {target}"
        );
    }
}

#[test]
fn an_image_and_a_form_no_reader_decodes_are_written_as_forms_drawing_nothing() {
    for (dict, role) in [
        (IMAGE, UndecodableRole::Image),
        (FORM, UndecodableRole::Form),
    ] {
        for profile in ["as-if-printed", "as-shown", "state-every-loss"] {
            for target in TARGETS {
                let source = painting(target, dict);
                let (report, output) = convert(&source, target, authorised_by(profile, target));
                let output = written(&report, output);
                assert_eq!(
                    decision(&report, ROW),
                    Decision::Authorised {
                        losses: Losses::one(Loss::UndecodableStream),
                        rewrite: Rewrite::UndecodableStreamRemoved,
                    },
                    "{profile} at {target}"
                );
                assert_eq!(
                    holds(&output, target).verdict(),
                    Verdict::Conforms,
                    "{profile} at {target}"
                );
                let written = object_six(&output);
                let form = written.as_stream().expect("a stream at the same name");
                assert_eq!(
                    form.dict.get("Subtype").and_then(Object::as_name),
                    Some(&Name::new(&b"Form"[..])),
                    "{profile} at {target}: a form"
                );
                assert!(form.data.is_empty(), "drawing nothing");
                assert!(form.dict.get("Filter").is_none(), "under no filter");
                let streams = &conversion(&report).undecodable_streams;
                assert_eq!(streams.len(), 1, "{streams:?}");
                assert_eq!(streams[0].role, role);
                assert_eq!(streams[0].filters, ["/XXXDecode"]);
                assert_eq!(streams[0].length, 3);
                let packet =
                    String::from_utf8(document_packet(&output)).expect("the packet is UTF-8");
                assert!(
                    packet.contains("(undecodable-stream)"),
                    "{profile} at {target}: xmpMM:History in the file names the loss"
                );
            }
        }
    }
}

#[test]
fn a_content_stream_no_reader_decodes_is_written_holding_no_operator() {
    for target in TARGETS {
        let source = Conforming {
            contents: Some(("/Filter /BrotliDecode".to_owned(), b"xyz".to_vec())),
            ..base(target)
        }
        .build();
        let (report, output) = convert(&source, target, authorised_by("as-if-printed", target));
        let output = written(&report, output);
        assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
        assert!(
            decoded_contents(&output).is_empty(),
            "no operator at {target}"
        );
        let streams = &conversion(&report).undecodable_streams;
        assert_eq!(streams.len(), 1, "{streams:?}");
        assert_eq!(streams[0].role, UndecodableRole::Content);
        assert_eq!(streams[0].filters, ["/BrotliDecode"]);
    }
}

#[test]
fn no_data_loss_refuses_an_undecodable_stream_by_the_loss_s_name() {
    for target in TARGETS {
        let source = painting(target, IMAGE);
        let (report, output) = convert(&source, target, authorised_by("no-data-loss", target));
        assert!(
            output.is_none(),
            "no-data-loss at {target}: nothing is written"
        );
        assert_eq!(
            decision(&report, ROW),
            Decision::Unauthorised {
                losses: Losses::one(Loss::UndecodableStream),
                rewrite: Rewrite::UndecodableStreamRemoved,
            },
            "no-data-loss at {target}"
        );
    }
}

#[test]
fn an_image_another_names_as_its_mask_keeps_the_refusal() {
    // A form in a mask's place is no mask (ISO 32000-2 §8.9.6), so what the window drew without
    // it is not decided and the row is refused rather than guessed (`doc/adr/2689`).
    let target = Target::Four(Flavour::Plain);
    let mask = "/Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray \
                /BitsPerComponent 8 /Filter /XXXDecode /Length 3";
    let image = format!("{IMAGE} /SMask 7 0 R /Length 1");
    let source = Conforming {
        objects: vec![
            String::from_utf8_lossy(&stream(&image, b"\x00")).into_owned(),
            String::from_utf8_lossy(&stream(mask, b"xyz")).into_owned(),
        ],
        resources: "/XObject << /X0 6 0 R >>".to_owned(),
        contents: Some((String::new(), b"q 10 0 0 10 0 0 cm /X0 Do Q".to_vec())),
        ..base(target)
    }
    .build();
    let (report, output) = convert(&source, target, authorised_by("as-if-printed", target));
    assert!(output.is_none(), "nothing is written");
    assert!(matches!(
        decision(&report, ROW),
        Decision::Refused(Because::NotBuiltYet(_))
    ));
}

#[test]
fn an_icc_profile_no_reader_decodes_keeps_the_refusal() {
    // The window draws the space as ISO 32000-2 §8.6.5.5's alternate; stating that alternate in
    // the profile's place, with ISO 19005's licence for a device space, is not built
    // (`doc/adr/2689`), so the row is refused by its own sentence.
    let target = Target::Four(Flavour::Plain);
    let profile = "/N 4 /Filter /XXXDecode /Length 3";
    let source = Conforming {
        objects: vec![String::from_utf8_lossy(&stream(profile, b"xyz")).into_owned()],
        resources: "/ColorSpace << /CS0 [/ICCBased 6 0 R] >>".to_owned(),
        contents: Some((String::new(), b"/CS0 cs 0 1 0 0 sc 0 0 10 10 re f".to_vec())),
        ..base(target)
    }
    .build();
    let (report, output) = convert(&source, target, authorised_by("as-if-printed", target));
    assert!(output.is_none(), "nothing is written");
    let Decision::Refused(Because::NotBuiltYet(because)) = decision(&report, ROW) else {
        panic!("{:?}", decision(&report, ROW));
    };
    assert!(because.contains("ICCBased"), "{because}");
}
