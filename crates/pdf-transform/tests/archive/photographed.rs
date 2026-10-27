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

/// Converts `source` with `text-as-image` and every font loss authorised, and returns what the
/// report lists as substituted, what it lists as photographed, and the output's bytes.
fn photographed_beside(source: &[u8]) -> (Vec<String>, usize, Vec<u8>) {
    let target = Target::Two(Level::B);
    let mut plan = photographing(target);
    plan.authorised.authorise(Loss::MachineFace);
    plan.authorised.authorise(Loss::SubstituteStretch);
    let (report, output) = convert_with_plan(source, &plan);
    let output = output.unwrap_or_else(|| {
        panic!(
            "the page is photographed: {:?}",
            decision(&report, "fonts/font-programs-embedded")
        )
    });
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let converted = conversion(&report);
    let substituted = converted
        .substituted
        .iter()
        .map(|font| font.requested.clone())
        .collect();
    (substituted, converted.photographed.len(), output)
}

/// Object 9, a simple font with no program and no descriptor, which the conversion embeds a
/// shipped face for (ISO 19005-2 section 6.2.11.4.1).
const HELVETICA: &str =
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>";

#[test]
fn a_font_only_the_photographed_content_selected_is_not_reported_embedded() {
    // The page's resources go with its content (`doc/adr/2394`), so a font only that content
    // selected is in no object the output holds: neither the report nor the file's
    // `xmpMM:History` may say a face was embedded for it (`doc/adr/2454`). A space reads back as
    // white space, so the owner's gate still admits the page (§9.10.2).
    let source = a_page_no_reader_can_read(
        "/F2 9 0 R",
        b"BT /F2 12 Tf 10 150 Td ( ) Tj ET",
        "",
        &[HELVETICA],
    );
    let (substituted, photographed, output) = photographed_beside(&source);
    assert_eq!(photographed, 1);
    assert!(substituted.is_empty(), "{substituted:?}");
    let written = String::from_utf8_lossy(&output);
    assert!(
        !written.contains("Helvetica"),
        "no font, no report and no history entry names it"
    );
}

#[test]
fn a_font_an_annotation_on_the_photographed_page_still_shows_is_reported_embedded() {
    // The control: the annotation stays an annotation over the photograph (ADR 2394), and its
    // appearance's resources still select the font, so the output carries it and its face, and
    // the report and the history say so.
    let appearance = "BT /F2 12 Tf 2 2 Td ( ) Tj ET";
    let source = a_page_no_reader_can_read(
        "/F2 9 0 R",
        b"BT /F2 12 Tf 10 150 Td ( ) Tj ET",
        "/Annots [11 0 R]",
        &[
            HELVETICA,
            &format!(
                "<< /Type /XObject /Subtype /Form /BBox [0 0 40 20] /Resources << /Font << /F2 9 \
                 0 R >> >> /Length {} >>\nstream\n{appearance}\nendstream",
                appearance.len() + 1
            ),
            "<< /Type /Annot /Subtype /Square /Rect [150 150 190 170] /F 4 /AP << /N 10 0 R >> >>",
        ],
    );
    let (substituted, photographed, output) = photographed_beside(&source);
    assert_eq!(photographed, 1);
    assert_eq!(substituted, ["Helvetica"]);
    let held = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
    let page = Pages::new(&held).get(0).expect("a first page");
    let annotations = held.get_key(&page.dict, "Annots");
    let annotation = held
        .resolve(&annotations)
        .as_array()
        .and_then(|annotations| annotations.first().cloned())
        .expect("the annotation stays");
    let annotation = held.resolve(&annotation);
    let appearances = held.get_key(annotation.as_dict().expect("a dictionary"), "AP");
    let normal = held.get_key(held.resolve(&appearances).as_dict().expect("an AP"), "N");
    let Object::Stream(normal) = held.resolve(&normal) else {
        panic!("the normal appearance is a stream");
    };
    let resources = held.get_key(&normal.dict, "Resources");
    let fonts = held.get_key(
        held.resolve(&resources).as_dict().expect("resources"),
        "Font",
    );
    let font = held.get_key(held.resolve(&fonts).as_dict().expect("fonts"), "F2");
    let font = held.resolve(&font);
    let descriptor = held.get_key(font.as_dict().expect("the font"), "FontDescriptor");
    let descriptor = held.resolve(&descriptor);
    assert!(
        descriptor
            .as_dict()
            .is_some_and(|descriptor| descriptor.get("FontFile2").is_some()
                || descriptor.get("FontFile").is_some()
                || descriptor.get("FontFile3").is_some()),
        "and the face reported is the face written: {descriptor:?}"
    );
}
