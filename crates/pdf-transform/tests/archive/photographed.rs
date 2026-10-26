//! A page showing an `Adobe-Identity` composite font no reader can name, photographed as the window
//! draws it (owner answer A570's second remedy, ADR 2394).
//!
//! The expected values come from ISO 32000-2: §9.7.5.2 (an identity `CMap` over a font the file does
//! not embed is the invalidity the photograph repairs), §9.10.2 (a font with no `/ToUnicode` and a
//! character collection of `Identity` names no character for its codes, so the page reads back
//! nothing), §8.3.2.3 (72 units to the inch, so the fixture's 200-unit page at 300 dpi is 833⅓
//! pixels, which `TargetSpec::for_page` rounds up to 834), §8.9.5 (an image fills the unit square of
//! the space `cm` sets up) and §7.7.3.3's `/Rotate`; and from A570's reading: the loss is
//! `text-as-image`, both shipped profiles carry it, and a page whose text a reader can extract is
//! never photographed.
use super::{Conforming, Decision, Level, Target, a_supplied_font, conversion, convert_with_plan};
use super::{decision, holds};
use pdf_archive::Verdict;
use pdf_model::Pages;
use pdf_syntax::object::Object;
use pdf_syntax::{Document, Limits};
use pdf_transform::archive::{Authorisations, Loss};
use pdf_transform::render::{Drawing, Sizing};

/// The fixture's page: a grey block in `CalGray`, then a string in `Identity-H` over an
/// `Adobe-Identity` `CIDFontType2` named `Arial` with no program and no `/ToUnicode`, then
/// `extra` — and `page` added to the page dictionary, `fonts` to its font resources and `more` as
/// objects 9 and up.
fn a_page_no_reader_can_read(fonts: &str, extra: &[u8], page: &str, more: &[&str]) -> Vec<u8> {
    Conforming {
        resources: format!(
            "/Font << /F1 6 0 R {fonts} >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
             [0.9505 1.0 1.089] >>] >>"
        ),
        page: page.to_owned(),
        contents: Some((
            String::new(),
            [
                &b"/CS0 cs 0.5 sc 20 30 120 40 re f BT /F1 12 Tf 10 100 Td <00240025> Tj ET "[..],
                extra,
            ]
            .concat(),
        )),
        objects: vec![
            "<< /Type /Font /Subtype /Type0 /BaseFont /Arial /Encoding /Identity-H \
             /DescendantFonts [7 0 R] >>"
                .to_owned(),
            "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Arial /CIDSystemInfo \
             << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 8 0 R \
             /DW 1000 /W [36 [667 667]] >>"
                .to_owned(),
            "<< /Type /FontDescriptor /FontName /Arial /Flags 32 \
             /FontBBox [-665 -325 2000 1006] /ItalicAngle 0 /Ascent 905 /Descent -212 \
             /CapHeight 716 /StemV 80 >>"
                .to_owned(),
        ]
        .into_iter()
        .chain(more.iter().map(|object| (*object).to_owned()))
        .collect(),
        ..Conforming::part_two()
    }
    .build()
}

/// A plan converting to `target` with `text-as-image` authorised and nothing else.
fn photographing(target: Target) -> pdf_transform::archive::ArchivePlan {
    let mut plan = a_supplied_font(target, "NotThisFont", &[]);
    let mut authorised = Authorisations::default();
    authorised.authorise(Loss::TextAsImage);
    plan.authorised = authorised;
    plan
}

/// The first page of `bytes` as the oracle backend draws it at 300 dpi.
fn drawn(bytes: &[u8]) -> pdf_render::Raster {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let page = Pages::new(&document).get(0).expect("a first page");
    pdf_transform::render::render_page(
        &document,
        &page,
        &pdf_model::view::ViewState::of(&document),
        &pdf_model::content::FontCache::new(),
        &mut render_cpu::CpuRasterizer::new(),
        Drawing {
            size: Sizing::Dpi(300.0),
            page_box: None,
            annotations: true,
        },
        &pdf_transform::Budget::default(),
    )
    .expect("the page draws")
    .raster
}

/// The largest difference of one channel between two rasters of one size.
fn largest_difference(left: &pdf_render::Raster, right: &pdf_render::Raster) -> u8 {
    assert_eq!(
        (left.width, left.height),
        (right.width, right.height),
        "one grid"
    );
    left.data
        .iter()
        .zip(&right.data)
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .unwrap_or(0)
}

/// Converts `source` with `text-as-image` authorised and holds the output to the target and to
/// the window's own drawing of the source.
fn photographed_and_drawn_alike(source: &[u8], target: Target) -> Vec<u8> {
    let (report, output) = convert_with_plan(source, &photographing(target));
    let Decision::Authorised { losses, .. } = decision(&report, "fonts/font-programs-embedded")
    else {
        panic!(
            "the page is photographed under the one loss: {:?}",
            decision(&report, "fonts/font-programs-embedded")
        );
    };
    assert!(losses.iter().eq([Loss::TextAsImage]), "{}", losses.words());
    let output = output.expect("a file is written");
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let pages = &conversion(&report).photographed;
    assert_eq!(pages.len(), 1, "{pages:?}");
    assert_eq!((pages[0].page, pages[0].dots_per_inch), (1, 300));
    assert_eq!(pages[0].fonts, ["Arial"]);
    // The sRGB profile read back as an `ICCBased` source is the identity to one level of 255
    // (`pdf_colour::icc`'s own test), and the image's samples are the source raster's at the same
    // grid, so the output drawn at 300 dpi is the source drawn at 300 dpi to that level.
    let difference = largest_difference(&drawn(source), &drawn(&output));
    assert!(
        difference <= 1,
        "the photograph moved a level by {difference}"
    );
    output
}

#[test]
fn a_page_no_reader_can_read_is_photographed_as_it_is_drawn() {
    let source = a_page_no_reader_can_read("", b"", "", &[]);
    let output = photographed_and_drawn_alike(&source, Target::Two(Level::B));
    let held = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
    let page = Pages::new(&held).get(0).expect("a first page");
    let images = held.get_key(&page.resources, "XObject");
    let images = held.resolve(&images);
    let image = held.get_key(
        images.as_dict().expect("an XObject dictionary"),
        "Photograph",
    );
    let Object::Stream(image) = held.resolve(&image) else {
        panic!("the photograph is an image stream");
    };
    assert_eq!(
        (
            held.get_key(&image.dict, "Width"),
            held.get_key(&image.dict, "Height")
        ),
        (Object::Integer(834), Object::Integer(834)),
        "200 units at 300 dpi, rounded up to whole pixels"
    );
    assert!(
        held.get_key(&page.resources, "Font").is_null(),
        "the page names no font"
    );
}

#[test]
fn a_quarter_turned_page_keeps_its_turn_and_is_drawn_alike() {
    let source = a_page_no_reader_can_read("", b"", "/Rotate 90", &[]);
    photographed_and_drawn_alike(&source, Target::Four(pdf_archive::Flavour::Plain));
}

#[test]
fn the_photograph_is_a_loss_a_caller_authorises() {
    let plan = a_supplied_font(Target::Two(Level::B), "NotThisFont", &[]);
    let (report, output) = convert_with_plan(&a_page_no_reader_can_read("", b"", "", &[]), &plan);
    assert!(output.is_none(), "no file is written");
    let Decision::Unauthorised { losses, .. } = decision(&report, "fonts/font-programs-embedded")
    else {
        panic!("the photograph waits for its authorisation");
    };
    assert!(losses.iter().eq([Loss::TextAsImage]), "{}", losses.words());
}

#[test]
fn a_page_whose_other_text_a_reader_can_extract_is_not_photographed() {
    // Object 9: Helvetica under WinAnsiEncoding, whose codes §9.10.2 names by their glyph names.
    let source = a_page_no_reader_can_read(
        "/F2 9 0 R",
        b"BT /F2 12 Tf 10 150 Td (Hi) Tj ET",
        "",
        &["<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"],
    );
    let (report, output) = convert_with_plan(&source, &photographing(Target::Two(Level::B)));
    assert!(output.is_none(), "no file is written");
    let Decision::Refused(because) = decision(&report, "fonts/font-programs-embedded") else {
        panic!("a page with extractable text is refused by name");
    };
    assert!(
        because.sentence().contains("text a reader can extract"),
        "{}",
        because.sentence()
    );
}

#[test]
fn level_u_keeps_the_refusal_the_font_had() {
    let source = a_page_no_reader_can_read("", b"", "", &[]);
    let mut plan = photographing(Target::Two(Level::U));
    plan.authorised.authorise(Loss::TextAsImage);
    let (report, output) = convert_with_plan(&source, &plan);
    assert!(output.is_none(), "no file is written");
    let Decision::Refused(because) = decision(&report, "fonts/font-programs-embedded") else {
        panic!("Level U is not offered a page with no text");
    };
    assert!(
        because.sentence().contains("no program was named for it"),
        "{}",
        because.sentence()
    );
}

#[test]
fn both_shipped_profiles_carry_the_photograph_and_as_shown_nothing_else_of_the_site() {
    for (name, machine_face) in [("as-if-printed", true), ("as-shown", false)] {
        let profile = pdf_transform::profiles::named(name).expect("a shipped profile");
        let configuration =
            pdf_transform::archive::Configuration::read(profile.text, Target::Two(Level::B))
                .expect("the shipped profile reads");
        let authorised = configuration.authorisations(Target::Two(Level::B));
        assert!(authorised.grants(Loss::TextAsImage), "{name}");
        assert_eq!(authorised.grants(Loss::MachineFace), machine_face, "{name}");
    }
}
