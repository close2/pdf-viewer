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

/// The profile of this renderer's assumed inks, which the archive writes for an uncharacterised
/// `DeviceCMYK` (ADR 2700), and the sRGB profile it writes for `DeviceRGB`.
const ASSUMED_INKS: &[u8] = include_bytes!("../../../../data/icc/quorra-assumed-inks.icc");
const SRGB: &[u8] = include_bytes!("../../../../data/icc/sRGB2014.icc");

/// A page filling a square in `[/ICCBased 6 0 R]`, object 6 the profile stream `dict` under
/// `/XXXDecode`, with the colour `colour`.
fn filling_in_a_profile(target: Target, dict: &str, colour: &str) -> Vec<u8> {
    let profile = format!("{dict} /Filter /XXXDecode /Length 3");
    Conforming {
        objects: vec![String::from_utf8_lossy(&stream(&profile, b"xyz")).into_owned()],
        resources: "/ColorSpace << /CS0 [/ICCBased 6 0 R] >>".to_owned(),
        contents: Some((
            String::new(),
            format!("/CS0 cs {colour} sc 0 0 10 10 re f").into_bytes(),
        )),
        ..base(target)
    }
    .build()
}

/// Page 1 as this reader draws it, on the processor, as RGBA bytes.
fn pixels(bytes: &[u8]) -> Vec<u8> {
    use pdf_render::{Rasterizer as _, TargetSpec};
    let document = Document::open(bytes.to_vec()).expect("an exact-xref fixture");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let state = pdf_model::view::ViewState::of(&document);
    let list = pdf_model::content::interpret_with(&document, &page, &state).display_list;
    let target = TargetSpec::for_page(&list, 2.0, 1 << 22).expect("a bounded target");
    render_cpu::CpuRasterizer::new()
        .rasterize(&list, target)
        .expect("CPU pixels")
        .data
}

/// The profile stream page 1's `/CS0` names in `output`.
fn the_profile(output: &[u8]) -> (Document, pdf_syntax::object::Dictionary, Vec<u8>) {
    let document =
        Document::open_with_limits(output.to_vec(), Limits::DEFAULT).expect("the output opens");
    let page = pdf_model::Pages::new(&document).get(0).expect("one page");
    let resources = document.get_key(&page.dict, "Resources");
    let spaces = document.get_key(resources.as_dict().expect("resources"), "ColorSpace");
    let space = document.get_key(spaces.as_dict().expect("a colour space table"), "CS0");
    let space = space.as_array().expect("an ICCBased array").to_vec();
    let Object::Stream(profile) = document.resolve(&space[1]) else {
        panic!("a profile stream");
    };
    let data = document
        .decoded_stream_data(&profile)
        .expect("it decodes")
        .to_vec();
    let dict = profile.dict.clone();
    (document, dict, data)
}

#[test]
fn an_icc_profile_no_reader_decodes_is_written_as_the_profile_of_the_space_it_was_drawn_in() {
    // ISO 32000-2 §8.6.5.5's Table 65: a profile the reader does not understand, with no
    // `/Alternate`, is drawn as DeviceRGB or DeviceCMYK by its `/N`. The profile written states
    // those colours as this reader draws them (`doc/adr/3211`): the assumed inks for four
    // components, sRGB for three, so the page draws what it drew.
    for (dict, colour, expected) in [
        ("/N 4", "0 1 0 0", ASSUMED_INKS),
        ("/N 4 /Alternate /DeviceCMYK", "1 0 0 0.2", ASSUMED_INKS),
        ("/N 3", "0.2 0.4 0.9", SRGB),
    ] {
        for profile in ["as-if-printed", "as-shown"] {
            for target in TARGETS {
                let source = filling_in_a_profile(target, dict, colour);
                let (report, output) = convert(&source, target, authorised_by(profile, target));
                let output = written(&report, output);
                assert_eq!(
                    decision(&report, ROW),
                    Decision::Authorised {
                        losses: Losses::one(Loss::UndecodableStream),
                        rewrite: Rewrite::UndecodableStreamRemoved,
                    },
                    "{dict}, {profile} at {target}"
                );
                assert_eq!(
                    holds(&output, target).verdict(),
                    Verdict::Conforms,
                    "{dict}, {profile} at {target}"
                );
                let (document, stated, data) = the_profile(&output);
                assert_eq!(data, expected, "{dict}, {profile} at {target}: the profile");
                assert_eq!(
                    document.get_key(&stated, "N").as_integer(),
                    Some(if expected.len() == SRGB.len() { 3 } else { 4 }),
                    "{dict}: its /N"
                );
                assert!(stated.get("Alternate").is_none(), "{dict}: no alternate");
                let streams = &conversion(&report).undecodable_streams;
                assert_eq!(streams.len(), 1, "{streams:?}");
                assert_eq!(streams[0].role, UndecodableRole::Profile);
                assert_eq!(
                    pixels(&output),
                    pixels(&source),
                    "{dict}, {profile} at {target}: drawn as the source was"
                );
            }
        }
    }
}

#[test]
fn an_icc_profile_of_one_component_or_another_alternate_keeps_the_refusal() {
    // DeviceGray has no profile this program ships, and an alternate other than the device
    // space of `/N` is drawn as that space, which no profile here states (`doc/adr/3211`).
    let target = Target::Four(Flavour::Plain);
    for (dict, colour) in [
        ("/N 1", "0.5"),
        (
            "/N 3 /Alternate [/CalRGB << /WhitePoint [0.9505 1 1.089] >>]",
            "0 1 0",
        ),
    ] {
        let source = filling_in_a_profile(target, dict, colour);
        let (report, output) = convert(&source, target, authorised_by("as-if-printed", target));
        assert!(output.is_none(), "{dict}: nothing is written");
        let Decision::Refused(Because::NotBuiltYet(because)) = decision(&report, ROW) else {
            panic!("{dict}: {:?}", decision(&report, ROW));
        };
        assert!(because.contains("ICCBased"), "{because}");
    }
}
