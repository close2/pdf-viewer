//! What an output intent this conversion adds does to the page this renderer draws, and that the
//! sentence the report carries for it says so (`doc/adr/2691`).
//!
//! ISO 19005-2 and ISO 19005-4 section 6.2.4.3 license a device colour space through a PDF/A
//! output intent whose destination profile is of that space's family, and a conforming reader
//! colour-manages the space through that profile. Where the file stated none, the source was drawn
//! without one: `DeviceRGB` as this processor's own sRGB (ADR 0009), and `DeviceCMYK` through its
//! assumed process inks (ADR 0263). The shipped sRGB profile draws the first to within a level of
//! 255; no ICC profile states the second, so the shipped press moves it, and the report's sentence
//! — `doc/adr/0927`'s condition on the rewrite — has to say which of the two happened.

use super::*;

/// ISO 19005-2 section 6.2.4.3's `DeviceCMYK` row.
const CMYK_ROW_PART_TWO: &str = "graphics/device-cmyk-needs-a-default-or-a-cmyk-output-intent";

/// ISO 19005-4 section 6.2.4.3's `DeviceCMYK` row.
const CMYK_UNDER_PART_FOUR: &str =
    "graphics/device-cmyk-needs-a-default-a-blending-space-or-a-cmyk-output-intent";

/// ISO 19005-2 section 6.2.4.3's `DeviceRGB` row.
const RGB_ROW_PART_TWO: &str = "graphics/device-rgb-needs-a-default-or-an-rgb-output-intent";

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

/// The largest difference of one channel between the two pages' pixels.
fn largest_difference(source: &[u8], output: &[u8]) -> u8 {
    let (before, after) = (drawn(source), drawn(output));
    assert_eq!((before.width, before.height), (after.width, after.height));
    before
        .data
        .iter()
        .zip(&after.data)
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap_or(0)
}

/// A part 2 page stating no output intent whose content is `contents`.
fn part_two_drawing(contents: &str) -> Vec<u8> {
    Conforming {
        contents: Some((String::new(), contents.as_bytes().to_vec())),
        ..Conforming::part_two()
    }
    .build()
}

/// Four ramps of 256 steps each, across the page: grey, then red, green and blue alone.
fn device_rgb_ramps() -> String {
    let mut contents = String::new();
    for (bottom, weights) in
        [0u8, 50, 100, 150]
            .into_iter()
            .zip([[1u8, 1, 1], [1, 0, 0], [0, 1, 0], [0, 0, 1]])
    {
        for step in 0..256u16 {
            let value = f32::from(step) / 255.0;
            let [red, green, blue] = weights.map(|weight| value * f32::from(weight));
            let left = f32::from(step) * 0.75;
            let _ = writeln!(
                contents,
                "{red} {green} {blue} rg {left} {bottom} 0.75 50 re f"
            );
        }
    }
    contents
}

/// The sentence the report states for `row`, which has to be the output intent's.
fn intent_sentence(report: &Report, row: &str) -> &'static str {
    let Decision::Stated {
        rewrite,
        reinterprets,
    } = decision(report, row)
    else {
        panic!("a statement: {:?}", decision(report, row));
    };
    assert_eq!(rewrite, Rewrite::OutputIntent);
    reinterprets
}

#[test]
fn the_shipped_rgb_intent_draws_every_device_rgb_colour_within_a_level_and_says_so() {
    // The shipped sRGB profile's primaries are the sRGB standard's, rounded to the profile's
    // s15Fixed16 numbers, and its curves are sampled; what is left of that, through this
    // renderer's route out of a profile, is at most one level (`colorspace_cos`, ADR 2691).
    let source = part_two_drawing(&device_rgb_ramps());
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
    let sentence = intent_sentence(&report, RGB_ROW_PART_TWO);
    assert!(
        sentence.contains("already showed those colours"),
        "{sentence}"
    );
    let largest = largest_difference(&source, &output);
    assert!(largest <= 1, "a DeviceRGB colour moved by {largest} levels");
}

#[test]
fn the_shipped_cmyk_intent_moves_this_renderers_drawing_and_the_report_says_so() {
    // A cyan this renderer drew through its assumed inks, and the press the shipped profile
    // describes, are different colours: the report may not claim this renderer showed it so
    // (`postscript_type4_many_outputs`, `issue9940`, ADR 2691).
    let source = part_two_drawing("1 0 0 0 k 0 0 200 200 re f");
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
    let sentence = intent_sentence(&report, CMYK_ROW_PART_TWO);
    assert!(
        !sentence.contains("already showed") && sentence.contains("assumed process inks"),
        "{sentence}"
    );
    assert!(
        sentence.contains("this program ships") && sentence.contains("--output-intent-profile"),
        "{sentence}"
    );
    assert!(
        largest_difference(&source, &output) > 1,
        "the sentence says the drawing moves, and it does"
    );
    // The same at part 4, whose row is answered by the same intent.
    let source = Conforming {
        contents: Some((String::new(), b"1 0 0 0 k 0 0 200 200 re f".to_vec())),
        ..Conforming::default()
    }
    .build();
    let (report, output) = convert(
        &source,
        Target::Four(Flavour::Plain),
        Authorisations::default(),
    );
    assert!(output.is_some(), "{}", conversion(&report).render());
    let sentence = intent_sentence(&report, CMYK_UNDER_PART_FOUR);
    assert!(sentence.contains("assumed process inks"), "{sentence}");
}

#[test]
fn a_cmyk_intent_the_operator_supplied_is_named_as_their_press() {
    let target = Target::Two(Level::B);
    let sinks = MemorySinks::new();
    let mut plan = plan_from("", target);
    plan.profile = Some(std::sync::Arc::from(PRESS));
    let report = apply(
        &Plan::Archive(plan),
        &[Source::new(part_two_drawing("1 0 0 0 k 0 0 200 200 re f"))],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the conversion applies");
    let sentence = intent_sentence(&report, CMYK_ROW_PART_TWO);
    assert!(
        sentence.contains("you supplied") && sentence.contains("assumed process inks"),
        "{sentence}"
    );
    assert!(!sentence.contains("already showed"), "{sentence}");
}

#[test]
fn a_cmyk_profile_the_file_already_held_keeps_the_sentence_it_was_drawn_through() {
    // A press-condition intent (§14.11.5's `GTS_PDFX`) is not the PDF/A one the row asks for, so
    // the conversion adds one naming the same profile object, as ISO 19005-2 section 6.2.3 requires.
    // The source was already drawn through that profile, so nothing this renderer showed moves.
    let mut hex = String::new();
    for byte in PRESS {
        let _ = write!(hex, "{byte:02X}");
    }
    hex.push('>');
    let source = Conforming {
        catalog: "/OutputIntents [7 0 R]".to_owned(),
        objects: vec![
            format!(
                "<< /N 4 /Filter /ASCIIHexDecode /Length {} >> stream\n{hex}\nendstream",
                hex.len()
            ),
            "<< /Type /OutputIntent /S /GTS_PDFX /OutputConditionIdentifier (Custom) \
             /DestOutputProfile 6 0 R >>"
                .to_owned(),
        ],
        contents: Some((String::new(), b"1 0 0 0 k 0 0 200 200 re f".to_vec())),
        ..Conforming::part_two()
    }
    .build();
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
    let sentence = intent_sentence(&report, CMYK_ROW_PART_TWO);
    assert!(
        sentence.contains("already showed those colours"),
        "{sentence}"
    );
    let largest = largest_difference(&source, &output);
    assert!(
        largest <= 1,
        "a DeviceCMYK colour moved by {largest} levels"
    );
}
