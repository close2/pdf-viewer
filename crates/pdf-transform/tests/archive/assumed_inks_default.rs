//! A part 4 file that draws in `DeviceCMYK` beside a destination profile of another family takes
//! the profile of this renderer's own assumed inks as its `/DefaultCMYK`, wherever nobody accepted
//! an assumed press (`doc/adr/2747`).
//!
//! ISO 19005-4 section 6.2.4.3 admits `DeviceCMYK` where a device independent `DefaultCMYK` is set
//! when the space is used, and section 6.2.3 lets a file state one destination profile, so beside
//! an RGB one that default is the licence left. ISO 32000-2 §8.6.5.6 passes the four values
//! unchanged to the default, so the profile decides what they look like: written over the
//! profile whose `A2B` samples the inks this renderer drew the source through (`doc/adr/2700`),
//! the page is drawn as it was, which is why the default is a statement and not a loss.

use super::*;

/// ISO 19005-4 section 6.2.4.3's `DeviceCMYK` row.
const ROW: &str = "graphics/device-cmyk-needs-a-default-a-blending-space-or-a-cmyk-output-intent";

/// ISO 19005-2 section 6.2.4.3's `DeviceCMYK` row.
const ROW_PART_TWO: &str = "graphics/device-cmyk-needs-a-default-or-a-cmyk-output-intent";

/// The profile of this renderer's assumed inks, as checked in.
const INKS: &[u8] = include_bytes!("../../../../data/icc/quorra-assumed-inks.icc");

/// A part 4 file with its own RGB intent that draws one black in `DeviceCMYK`.
fn source() -> Vec<u8> {
    with_its_own_rgb_intent(Conforming::default(), "", "0 0 0 1 k 0 0 10 10 re f")
}

/// The profile stream the output's page `/DefaultCMYK` names, decoded.
fn default_profile(output: &[u8]) -> Vec<u8> {
    let document = Document::open_with_limits(output.to_vec(), Limits::DEFAULT).expect("it opens");
    let space = default_cmyk(&document);
    assert_eq!(
        space
            .first()
            .and_then(pdf_syntax::Object::as_name)
            .map(pdf_syntax::object::Name::as_bytes),
        Some(&b"ICCBased"[..]),
        "{space:?}"
    );
    let stream = document.resolve(space.get(1).expect("a profile stream"));
    let stream = stream.as_stream().expect("a stream");
    assert_eq!(document.get_key(&stream.dict, "N").as_integer(), Some(4));
    document
        .decoded_stream_data(stream)
        .expect("the profile decodes")
        .to_vec()
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
fn an_unauthorised_part_four_file_takes_the_assumed_inks_as_its_default_cmyk() {
    // Nothing authorised, nothing supplied: section 6.2.4.3's device independent DefaultCMYK is
    // written over the assumed inks' profile, the file's own RGB intent stays the only intent, the
    // decision is a statement naming this renderer, and the history records the default.
    let target = Target::Four(Flavour::Plain);
    let (report, output) = convert(&source(), target, Authorisations::default());
    let Decision::Stated {
        rewrite,
        reinterprets,
    } = decision(&report, ROW)
    else {
        panic!("a statement: {:?}", decision(&report, ROW));
    };
    assert_eq!(rewrite, Rewrite::SuppliedDefaultCmyk);
    assert!(
        reinterprets.contains("inks this renderer assumes") && !reinterprets.contains("supplied"),
        "{reinterprets}"
    );
    let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
    assert_eq!(
        holds(&output, target).verdict(),
        Verdict::Conforms,
        "{}",
        holds(&output, target).render()
    );
    assert_eq!(
        default_profile(&output),
        INKS,
        "the assumed inks, byte for byte"
    );
    let document = Document::open_with_limits(output.clone(), Limits::DEFAULT).expect("it opens");
    let catalog = document.catalog().expect("a catalog");
    let intents = document.get_key(&catalog, "OutputIntents");
    assert_eq!(
        intents.as_array().map(<[_]>::len),
        Some(1),
        "no second intent"
    );
    assert!(
        conversion(&report).press_assumed.is_none(),
        "no press is assumed"
    );
    let packet = String::from_utf8(document_packet(&output)).expect("the packet is UTF-8");
    assert!(
        packet.contains("the process inks this renderer assumes")
            && !packet.contains("press-assumed"),
        "{packet}"
    );
}

#[test]
fn the_assumed_inks_draw_the_page_as_the_window_drew_it() {
    // ISO 32000-2 §8.6.5.6 hands the four values unchanged to the default, so the page is drawn
    // through the profile's A2B where the source was drawn through the ink cube that table
    // samples. ADR 2700 measured that table within 0.88 of a level of 255 on every one-ink ramp and
    // 0.79 at the press grid and its cell midpoints; rounded to whole levels, one level. The patches are
    // each ink's ramp at sixteenths and every mix of four inks at 0, 0.3, 0.7 and 1.
    let mut content = String::new();
    let mut patches: Vec<[f32; 4]> = Vec::new();
    for ink in 0..4 {
        for step in 0..=16u8 {
            let mut colour = [0.0; 4];
            colour[ink] = f32::from(step) / 16.0;
            patches.push(colour);
        }
    }
    let levels = [0.0, 0.3, 0.7, 1.0];
    for c in levels {
        for m in levels {
            for y in levels {
                for k in levels {
                    patches.push([c, m, y, k]);
                }
            }
        }
    }
    for (at, [c, m, y, k]) in patches.iter().enumerate() {
        let (column, row) = (at % 20, at / 20);
        let _ = write!(
            content,
            "{c} {m} {y} {k} k {} {} 10 10 re f ",
            column * 10,
            row * 10
        );
    }
    let source = with_its_own_rgb_intent(Conforming::default(), "", &content);
    let (report, output) = convert(
        &source,
        Target::Four(Flavour::Plain),
        Authorisations::default(),
    );
    let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
    let (before, after) = (drawn(&source), drawn(&output));
    assert_eq!((before.width, before.height), (after.width, after.height));
    let largest = before
        .data
        .iter()
        .zip(&after.data)
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap_or(0);
    println!("assumed inks: largest channel difference {largest} of 255");
    assert!(largest <= 1, "the page moved by {largest} levels");
}

#[test]
fn the_configured_press_does_not_stand_in_for_the_inks() {
    // An operator's configured press stands in for the shipped one, and for nothing else
    // (`doc/questions/A784`, ADR 2705): without press-assumed the inks are the default.
    let target = Target::Four(Flavour::Plain);
    let mut plan = plan_from("", target);
    plan.press = Some(std::sync::Arc::from(PRESS));
    let (report, output) = convert_with_plan(&source(), &plan);
    let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
    assert_eq!(default_profile(&output), INKS);
    assert!(conversion(&report).press_assumed.is_none());
}

#[test]
fn part_two_keeps_the_device_n_default() {
    // ISO 19005-2 section 6.2.4.3 admits a DeviceN-based DefaultCMYK, which states no press, so
    // part 2's row is answered by it and the inks are not offered there.
    let target = Target::Two(Level::B);
    let source = with_its_own_rgb_intent(Conforming::part_two(), "", "0 0 0 1 k 0 0 10 10 re f");
    let (report, output) = convert(&source, target, Authorisations::default());
    assert!(output.is_some(), "{}", conversion(&report).render());
    assert_eq!(
        decision(&report, ROW_PART_TWO).rewrite(),
        Some(Rewrite::DefaultCmyk),
        "{:?}",
        decision(&report, ROW_PART_TWO)
    );
}

#[test]
fn overprint_mode_one_keeps_the_refusal_for_the_inks_too() {
    // The default makes a DeviceCMYK mark an ICCBased CMYK one (§8.6.5.6), and ISO 19005-4 section
    // 6.2.4.2 forbids overprint mode 1 with overprinting on for it; the guard of ADR 2518 binds
    // every press, this one included.
    let fixture = Conforming {
        resources: "/ExtGState << /G0 << /OP true /op true /OPM 1 >> >>".to_owned(),
        ..Conforming::default()
    };
    let source = with_its_own_rgb_intent(fixture, "", "/G0 gs 0 0 0 1 k 0 0 10 10 re f");
    let (report, output) = convert(
        &source,
        Target::Four(Flavour::Plain),
        Authorisations::default(),
    );
    assert!(output.is_none(), "nothing is written");
    let sentence = not_built_sentence(&report, ROW);
    assert!(sentence.contains("overprint mode 1"), "{sentence}");
}
