//! A part 4 file that draws in `DeviceCMYK` beside an RGB destination profile converts behind the
//! loss `press-assumed` (`doc/adr/2518`).
//!
//! ISO 19005-4 section 6.2.4.3 licenses `DeviceCMYK` through a CMYK destination profile or a
//! device independent `DefaultCMYK`, and section 6.2.3 lets the file state one destination
//! profile, here its own RGB one. The licence left is an ICC CMYK `DefaultCMYK`, which states a
//! press the file does not name; the profile this program ships is written as it only where the
//! caller authorised the loss, and an operator's own profile outranks it.

use super::*;

/// The row part 4 fails for a `DeviceCMYK` use no licence covers.
const ROW: &str = "graphics/device-cmyk-needs-a-default-a-blending-space-or-a-cmyk-output-intent";

/// What the shipped `as-if-printed` profile says, as a configuration for `target`.
fn shipped(name: &str, target: Target) -> pdf_transform::archive::Configuration {
    let profile = pdf_transform::profiles::named(name).expect("a shipped profile");
    pdf_transform::archive::Configuration::read(profile.text, target).expect("it reads")
}

/// A part 4 file with its own RGB intent that draws one black in `DeviceCMYK`.
fn source() -> Vec<u8> {
    with_its_own_rgb_intent(Conforming::default(), "", "0 0 0 1 k 0 0 10 10 re f")
}

/// The authorisation this loss stands for, and nothing else.
fn authorised() -> Authorisations {
    let mut authorised = Authorisations::default();
    authorised.authorise(Loss::PressAssumed);
    authorised
}

#[test]
fn an_authorised_assumed_press_is_written_as_the_default_cmyk_and_named() {
    let target = Target::Four(Flavour::Plain);
    let (report, output) = convert(&source(), target, authorised());
    let Decision::Authorised { losses, rewrite } = decision(&report, ROW) else {
        panic!("a loss, authorised: {:?}", decision(&report, ROW));
    };
    assert_eq!(rewrite, Rewrite::SuppliedDefaultCmyk);
    assert!(losses.iter().eq([Loss::PressAssumed]));
    let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
    assert_eq!(
        holds(&output, target).verdict(),
        Verdict::Conforms,
        "{}",
        holds(&output, target).render()
    );
    let document = Document::open_with_limits(output.clone(), Limits::DEFAULT).expect("it opens");
    let space = default_cmyk(&document);
    let stream = document.resolve(space.get(1).expect("a profile stream"));
    let stream = stream.as_stream().expect("a stream");
    assert_eq!(document.get_key(&stream.dict, "N").as_integer(), Some(4));
    assert_eq!(
        document.decoded_stream_data(stream).as_deref(),
        Some(PRESS),
        "the profile is the one this program ships, byte for byte"
    );
    let catalog = document.catalog().expect("a catalog");
    let intents = document.get_key(&catalog, "OutputIntents");
    assert_eq!(
        intents.as_array().map(<[_]>::len),
        Some(1),
        "no second intent"
    );

    // Named in the report, in both of its forms, and in the file's own history.
    let named = conversion(&report)
        .press_assumed
        .as_ref()
        .expect("the report names the press");
    assert_eq!(named.source, pdf_transform::archive::ProfileSource::Shipped);
    // The history's sentence names this description as a literal; the profile's own `desc` tag is
    // what the report prints, so the two cannot drift apart unseen.
    assert_eq!(named.describes.as_deref(), Some("GRACoL2006_Coated1v2.icc"));
    let text = conversion(&report).render();
    assert!(text.contains("press-assumed"), "{text}");
    assert!(text.contains("not an imitation of printing"), "{text}");
    let json = conversion(&report).to_json().render();
    assert!(json.contains("\"press_assumed\""), "{json}");
    let packet = String::from_utf8(document_packet(&output)).expect("the packet is UTF-8");
    assert!(
        packet.contains("GRACoL2006_Coated1v2.icc")
            && packet.contains("press-assumed")
            && packet.contains("an assumed press")
            && packet.contains("no operator supplied one"),
        "{packet}"
    );
    assert!(
        !packet.contains("was made for"),
        "the history claims no provenance for the press: {packet}"
    );
    // What the report says was written is true of this route: nobody supplied the press.
    let described = rewrite.describe();
    assert!(
        !described.contains("the operator supplied") && described.contains("press-assumed"),
        "{described}"
    );
}

#[test]
fn nothing_authorised_keeps_the_refusal_and_it_names_the_loss() {
    let (report, output) = convert(
        &source(),
        Target::Four(Flavour::Plain),
        Authorisations::default(),
    );
    assert!(output.is_none(), "nothing is written");
    let sentence = not_built_sentence(&report, ROW);
    assert!(
        sentence.contains("press-assumed") && sentence.contains("--output-intent-profile"),
        "{sentence}"
    );
    assert!(conversion(&report).press_assumed.is_none());
}

#[test]
fn a_press_the_operator_supplied_outranks_the_assumed_one_and_costs_no_loss() {
    let target = Target::Four(Flavour::Plain);
    let sinks = MemorySinks::new();
    let mut plan = plan_from("", target);
    plan.authorised = authorised();
    plan.profile = Some(std::sync::Arc::from(PRESS));
    let report = apply(
        &Plan::Archive(plan),
        &[Source::new(source())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the conversion applies");
    let Decision::Stated {
        rewrite,
        reinterprets,
    } = decision(&report, ROW)
    else {
        panic!("the operator's statement: {:?}", decision(&report, ROW));
    };
    assert_eq!(rewrite, Rewrite::SuppliedDefaultCmyk);
    assert!(reinterprets.contains("you supplied"), "{reinterprets}");
    assert!(conversion(&report).press_assumed.is_none());
}

#[test]
fn part_two_is_not_offered_the_assumed_press() {
    // Part 2's other licence is the DeviceN default, which states no press, and a group blending in
    // DeviceCMYK beside an RGB intent has no licence there without an operator's profile
    // (`doc/adr/2130`); the authorisation changes nothing about it.
    let source = Conforming {
        page: "/Group << /S /Transparency /CS /DeviceCMYK >>".to_owned(),
        contents: Some(paints_in_device_rgb()),
        ..Conforming::part_two()
    }
    .build();
    let (report, output) = convert(&source, Target::Two(Level::B), authorised());
    assert!(output.is_none(), "nothing is written");
    assert!(conversion(&report).press_assumed.is_none());
}

#[test]
fn the_shipped_profiles_say_which_of_them_assumes_the_press() {
    // ISO 19005-4 section 6.2.4.3's row binds part 4 alone, so a `discard` at it authorises the
    // loss there and nowhere else; `as-shown` leaves the site at its default, `stop` (A694 item 2).
    let part_four = Target::Four(Flavour::Plain);
    assert!(
        shipped("as-if-printed", part_four)
            .authorisations(part_four)
            .grants(Loss::PressAssumed)
    );
    assert!(
        !shipped("as-if-printed", Target::Two(Level::B))
            .authorisations(Target::Two(Level::B))
            .grants(Loss::PressAssumed)
    );
    assert!(
        !shipped("as-shown", part_four)
            .authorisations(part_four)
            .grants(Loss::PressAssumed)
    );
}

/// The page as this program's window draws it at 72 dots to the inch.
fn drawn(bytes: &[u8]) -> pdf_render::Raster {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = pdf_model::Pages::new(&document)
        .get(0)
        .expect("a first page");
    pdf_transform::render::render_page(
        &document,
        &page,
        &pdf_model::view::ViewState::of(&document),
        &pdf_model::content::FontCache::new(),
        &mut render_cpu::CpuRasterizer::new(),
        pdf_transform::render::Drawing {
            size: pdf_transform::render::Sizing::Dpi(72.0),
            page_box: None,
            annotations: true,
        },
        &Budget::default(),
    )
    .expect("the page draws")
    .raster
}

#[test]
fn overprint_mode_one_behind_the_default_keeps_the_refusal() {
    // ISO 32000-2 §8.6.5.6 makes a DefaultCMYK the space of every DeviceCMYK colour, so a mark
    // painted `0 0 0 1 k` with overprinting on and `/OPM 1` becomes an ICCBased CMYK mark, which
    // ISO 19005-4 section 6.2.4.2 forbids. veraPDF fails such an output on that clause where this
    // tree's own check does not see it, so the press is not assumed for the file.
    let fixture = Conforming {
        resources: "/ExtGState << /G0 << /OP true /op true /OPM 1 >> >>".to_owned(),
        ..Conforming::default()
    };
    let source = with_its_own_rgb_intent(fixture, "", "/G0 gs 0 0 0 1 k 0 0 10 10 re f");
    let (report, output) = convert(&source, Target::Four(Flavour::Plain), authorised());
    assert!(output.is_none(), "nothing is written");
    let sentence = not_built_sentence(&report, ROW);
    assert!(sentence.contains("overprint mode 1"), "{sentence}");
    assert!(conversion(&report).press_assumed.is_none());
}

#[test]
fn the_loss_is_a_real_shift_of_the_colours_and_no_mark_moves() {
    // The loss is the press: the window draws `DeviceCMYK` through its assumed inks, and the output
    // says the colours are the shipped profile's. A loss that moved nothing would be a statement
    // and not a loss, so the pixels must differ; and the content stream, which this converter
    // never edits (`doc/adr/0816`), is the producer's bytes, so the same cells are painted.
    let content = "1 0 0 0 k 0 0 10 10 re f 0 0 0 1 k 10 0 10 10 re f";
    let source = with_its_own_rgb_intent(Conforming::default(), "", content);
    let (_, output) = convert(&source, Target::Four(Flavour::Plain), authorised());
    let output = output.expect("a file is written");
    let (before, after) = (drawn(&source), drawn(&output));
    assert_eq!((before.width, before.height), (after.width, after.height));
    let largest = before
        .data
        .iter()
        .zip(&after.data)
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap_or(0);
    let unpainted_moved = before
        .data
        .chunks(4)
        .zip(after.data.chunks(4))
        .filter(|(a, b)| a.iter().all(|c| *c == 255) != b.iter().all(|c| *c == 255))
        .count();
    println!(
        "press-assumed: largest channel difference {largest} of 255; cells that gained or lost paint {unpainted_moved}"
    );
    assert!(largest > 0, "the colours are now the press's");
    assert_eq!(unpainted_moved, 0, "no mark moves");
}

/// A part 4 file with its own RGB intent whose page draws in `DeviceCMYK` and whose deepest form
/// of `depth` nested ones paints a `DeviceCMYK` mark with overprinting on and overprint mode 1.
fn mode_one_in_a_form_chain(depth: usize) -> Vec<u8> {
    let mut hex = String::new();
    for byte in SRGB {
        let _ = write!(hex, "{byte:02X}");
    }
    hex.push('>');
    let mut objects = vec![
        format!(
            "<< /N 3 /Filter /ASCIIHexDecode /Length {} >> stream\n{hex}\nendstream",
            hex.len()
        ),
        "<< /Type /OutputIntent /S /GTS_PDFA1 /OutputConditionIdentifier (sRGB) \
         /DestOutputProfile 6 0 R >>"
            .to_owned(),
    ];
    // Object 8 is the outermost form; each names the next, and the last paints.
    for level in 0..depth {
        let (resources, content) = if level.saturating_add(1) == depth {
            (
                "/ExtGState << /G0 << /OP true /op true /OPM 1 >> >>".to_owned(),
                "/G0 gs 0 0 0 1 k 0 0 5 5 re f",
            )
        } else {
            (
                format!("/XObject << /X {} 0 R >>", level.saturating_add(9)),
                "/X Do",
            )
        };
        objects.push(format!(
            "<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] /Resources << {resources} >> \
             /Length {} >> stream\n{content}\nendstream",
            content.len()
        ));
    }
    Conforming {
        catalog: "/OutputIntents [7 0 R]".to_owned(),
        resources: "/XObject << /X 8 0 R >>".to_owned(),
        objects,
        contents: Some((String::new(), b"0 0 0 1 k 0 0 10 10 re f /X Do".to_vec())),
        ..Conforming::default()
    }
    .build()
}

#[test]
fn a_walk_a_bound_stopped_does_not_let_a_mode_one_mark_through() {
    // The survey stops following forms at a depth, so a mark below it is unseen and the absence of
    // a finding says nothing: the guard refuses a walk that stopped short, as `overprint::zeroing`
    // does, and it refuses it for an operator's press as for the assumed one (`doc/adr/2518`).
    let target = Target::Four(Flavour::Plain);
    let shallow = mode_one_in_a_form_chain(2);
    let (report, output) = convert(&shallow, target, authorised());
    assert!(output.is_none(), "the walked mark is refused");
    assert!(not_built_sentence(&report, ROW).contains("overprint mode 1"));
    let deep = mode_one_in_a_form_chain(24);
    let (report, output) = convert(&deep, target, authorised());
    assert!(output.is_none(), "{}", conversion(&report).render());
    assert!(not_built_sentence(&report, ROW).contains("overprint mode 1"));
}

#[test]
fn an_operators_press_does_not_reach_a_mode_one_mark_either() {
    // The refusal does not send the operator to a route that converts a file veraPDF fails on
    // ISO 19005-4 section 6.2.4.2 and the tree's own check passes.
    let target = Target::Four(Flavour::Plain);
    let source = with_its_own_rgb_intent(
        Conforming {
            resources: "/ExtGState << /G0 << /OP true /op true /OPM 1 >> >>".to_owned(),
            ..Conforming::default()
        },
        "",
        "/G0 gs 0 0 0 1 k 0 0 10 10 re f",
    );
    let sinks = MemorySinks::new();
    let mut plan = plan_from("", target);
    plan.profile = Some(std::sync::Arc::from(PRESS));
    let report = apply(
        &Plan::Archive(plan),
        &[Source::new(source)],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the conversion applies");
    assert!(
        matches!(decision(&report, ROW), Decision::Refused(_)),
        "{:?}",
        decision(&report, ROW)
    );
    assert!(not_built_sentence(&report, ROW).contains("overprint mode 1"));
}
