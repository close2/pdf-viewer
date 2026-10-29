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
use std::fmt::Write as _;

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

// ---------------------------------------------------------------------------------------------
// The wider gate (A694 item 1, ADR 2516): `as-if-printed` photographs a page whatever text it shows
// ---------------------------------------------------------------------------------------------
//
// Expected values: §9.10.2 (a Type 1 font under `WinAnsiEncoding` names its characters, so the
// fixture's string reads back as `Hello`), ISO 32000-2 Annex A (the operator summary the base
// standard defines, which `Frobnicate` is not on) and A694 (the loss is `text-as-image` per page,
// the page's text is named in the report, `as-shown`, Level U and Level A keep the refusal).

/// A page showing `Hello` in Helvetica and then an operator the base standard does not define.
fn a_page_with_text_and_an_undefined_operator() -> Vec<u8> {
    Conforming {
        resources: "/Font << /F1 6 0 R >>".to_owned(),
        contents: Some((
            String::new(),
            b"BT /F1 12 Tf 10 100 Td (Hello) Tj ET 1 2 Frobnicate".to_vec(),
        )),
        objects: vec![HELVETICA.to_owned()],
        ..Conforming::part_two()
    }
    .build()
}

/// The authorisations a shipped profile states for `target`.
fn authorised_by(profile: &str, target: Target) -> Authorisations {
    let profile = pdf_transform::profiles::named(profile).expect("a shipped profile");
    pdf_transform::archive::Configuration::read(profile.text, target)
        .expect("the shipped profile reads")
        .authorisations(target)
}

/// A plan converting to `target` under what `profile` authorises.
fn printing(profile: &str, target: Target) -> pdf_transform::archive::ArchivePlan {
    let mut plan = a_supplied_font(target, "NotThisFont", &[]);
    plan.authorised = authorised_by(profile, target);
    plan
}

const OPERATORS: &str = "graphics/only-operators-the-base-standard-defines";

#[test]
fn a_page_with_a_fault_of_its_content_stream_is_photographed_under_as_if_printed() {
    let source = a_page_with_text_and_an_undefined_operator();
    let target = Target::Two(Level::B);
    let (report, output) = convert_with_plan(&source, &printing("as-if-printed", target));
    let Decision::Authorised { losses, .. } = decision(&report, OPERATORS) else {
        panic!("{:?}", decision(&report, OPERATORS));
    };
    assert!(
        losses.iter().eq([Loss::TextAsImage, Loss::PageAsImage]),
        "{}",
        losses.words()
    );
    let output = output.expect("a file is written");
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let pages = &conversion(&report).photographed;
    assert_eq!(pages.len(), 1, "{pages:?}");
    assert_eq!(pages[0].page, 1);
    assert_eq!(pages[0].text, "Hello");
    assert_eq!(pages[0].requirements, [OPERATORS]);
    assert!(pages[0].anywhere);
    // The text the photograph turned into pixels is named in the file's own history and in the
    // JSON report, and the operator is in no content stream of the output.
    let recorded = conversion(&report)
        .recorded
        .as_ref()
        .expect("a history entry");
    assert!(recorded.contains("\"Hello\""), "{recorded}");
    let packet = String::from_utf8(super::document_packet(&output)).expect("the packet is UTF-8");
    assert!(
        packet.contains("Hello"),
        "the file's own history names the text:\n{packet}"
    );
    let json = conversion(&report).to_json().render();
    assert!(json.contains("\"text\": \"Hello\""), "{json}");
    assert!(json.contains("page-as-image"), "{json}");
    // The photograph is the window's drawing of the source at 300 dpi.
    let difference = largest_difference(&drawn(&source), &drawn(&output));
    assert!(
        difference <= 1,
        "the photograph moved a level by {difference}"
    );
}

/// A page whose text and undefined operator share a content stream with `extra`.
fn a_page_with_a_fault_and(extra: &[u8], objects: Vec<String>, resources: &str) -> Vec<u8> {
    let mut content = b"BT /F1 12 Tf 10 100 Td (Hello) Tj ET 1 2 Frobnicate ".to_vec();
    content.extend_from_slice(extra);
    let mut all = vec![HELVETICA.to_owned()];
    all.extend(objects);
    Conforming {
        resources: format!("/Font << /F1 6 0 R >> {resources}"),
        contents: Some((String::new(), content)),
        objects: all,
        ..Conforming::part_two()
    }
    .build()
}

#[test]
fn a_fault_of_the_page_s_own_content_is_photographed_as_the_window_draws_it() {
    // `re` wants four operands; the window skips it and draws the rest (ISO 32000-2 section 7.8.2).
    let source = a_page_with_a_fault_and(b"10 re f", Vec::new(), "");
    let target = Target::Two(Level::B);
    let (report, output) = convert_with_plan(&source, &printing("as-if-printed", target));
    let output = output.unwrap_or_else(|| panic!("{:?}", decision(&report, OPERATORS)));
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    assert_eq!(conversion(&report).photographed.len(), 1);
}

#[test]
fn a_page_the_window_could_not_draw_whole_is_not_photographed() {
    // An image whose samples are missing: the window draws a page without the picture, and
    // a photograph would record that gap of this reader as the document's loss.
    let image = String::from_utf8_lossy(&super::stream(
        "/Type /XObject /Subtype /Image /Width 4 /Height 4 /ColorSpace /DeviceGray \
         /BitsPerComponent 8 /Length 1",
        b"\x00",
    ))
    .into_owned();
    let source = a_page_with_a_fault_and(
        b"q 10 0 0 10 0 0 cm /Im0 Do Q",
        vec![image],
        "/XObject << /Im0 7 0 R >>",
    );
    let (report, output) =
        convert_with_plan(&source, &printing("as-if-printed", Target::Two(Level::B)));
    assert!(output.is_none(), "no file is written");
    assert!(conversion(&report).photographed.is_empty());
    assert!(matches!(
        decision(&report, OPERATORS),
        Decision::Refused(pdf_transform::archive::Because::TheFence(_))
    ));
}

#[test]
fn as_shown_keeps_the_refusal_of_a_fault_of_the_content_stream() {
    let source = a_page_with_text_and_an_undefined_operator();
    let (report, output) = convert_with_plan(&source, &printing("as-shown", Target::Two(Level::B)));
    assert!(output.is_none(), "no file is written");
    assert!(matches!(
        decision(&report, OPERATORS),
        Decision::Refused(pdf_transform::archive::Because::TheFence(_))
    ));
}

#[test]
fn nothing_is_photographed_where_the_loss_was_not_authorised() {
    let source = a_page_with_text_and_an_undefined_operator();
    let mut plan = printing("as-if-printed", Target::Two(Level::B));
    plan.authorised.page_as_image = false;
    let (report, output) = convert_with_plan(&source, &plan);
    assert!(output.is_none(), "no file is written");
    assert!(matches!(
        decision(&report, OPERATORS),
        Decision::Refused(pdf_transform::archive::Because::TheFence(_))
    ));
}

#[test]
fn levels_u_and_a_keep_the_refusal_of_a_fault_of_the_content_stream() {
    let source = a_page_with_text_and_an_undefined_operator();
    for level in [Level::U, Level::A] {
        let (report, output) =
            convert_with_plan(&source, &printing("as-if-printed", Target::Two(level)));
        assert!(output.is_none(), "no file is written at {level:?}");
        assert!(
            matches!(
                decision(&report, OPERATORS),
                Decision::Refused(pdf_transform::archive::Because::TheFence(_))
            ),
            "{level:?}"
        );
        assert!(conversion(&report).photographed.is_empty());
    }
}

#[test]
fn every_part_four_flavour_photographs_a_page_under_as_if_printed() {
    let source = Conforming {
        target: Target::Four(pdf_archive::Flavour::Plain),
        resources: "/Font << /F1 6 0 R >>".to_owned(),
        contents: Some((
            String::new(),
            b"BT /F1 12 Tf 10 100 Td (Hello) Tj ET 1 2 Frobnicate".to_vec(),
        )),
        objects: vec![HELVETICA.to_owned()],
        ..Conforming::default()
    }
    .build();
    let target = Target::Four(pdf_archive::Flavour::Plain);
    let (report, output) = convert_with_plan(&source, &printing("as-if-printed", target));
    let output = output.expect("a file is written");
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    assert_eq!(conversion(&report).photographed.len(), 1);
}

#[test]
fn the_text_a_report_names_is_cut_where_a_page_holds_more_than_the_bound() {
    let text = "word ".repeat(pdf_transform::archive::MAX_TEXT_CHARS);
    let shown = format!(
        "BT /F1 12 Tf 10 100 Td ({}) Tj ET 1 2 Frobnicate",
        text.trim_end()
    );
    let source = Conforming {
        resources: "/Font << /F1 6 0 R >>".to_owned(),
        contents: Some((String::new(), shown.into_bytes())),
        objects: vec![HELVETICA.to_owned()],
        ..Conforming::part_two()
    }
    .build();
    let (report, _) = convert_with_plan(&source, &printing("as-if-printed", Target::Two(Level::B)));
    let pages = &conversion(&report).photographed;
    assert_eq!(pages.len(), 1, "{pages:?}");
    assert!(pages[0].text.chars().count() <= pdf_transform::archive::MAX_TEXT_CHARS);
    assert!(pages[0].text_elided > 0, "{}", pages[0].text_elided);
}

#[test]
fn a_photograph_is_in_the_files_own_history_at_level_b_whatever_else_the_packet_needs() {
    // The fixture's packet already states Level B's identification, so nothing else asks for the
    // packet to be edited and the loss is recorded for its own sake (ADR 2394 item 5).
    let source = a_page_no_reader_can_read("", b"", "", &[]);
    let output = photographed_and_drawn_alike(&source, Target::Two(Level::B));
    let packet = String::from_utf8(super::document_packet(&output)).expect("the packet is UTF-8");
    assert!(packet.contains("xmpMM:History"), "{packet}");
    assert!(packet.contains("losing text-as-image"), "{packet}");
}

/// `bytes` with `faulty` more pages that share the first page's content and `clean` more that show
/// one word, added by an incremental update (§7.5.6) of the page tree.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a fixture builder: object numbers and counts a few dozen at most, no input is hostile"
)]
fn with_more_pages(mut bytes: Vec<u8>, faulty: usize, clean: usize) -> Vec<u8> {
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let previous = text
        .rsplit_once("startxref\n")
        .expect("xref")
        .1
        .lines()
        .next()
        .expect("offset")
        .to_owned();
    let first_new = 11_usize;
    let pages = faulty + clean;
    let content = first_new + pages;
    let kids = (0..pages).fold(String::new(), |mut kids, index| {
        let _ = write!(kids, " {} 0 R", first_new + index);
        kids
    });
    let mut offsets = Vec::new();
    let tree = bytes.len();
    bytes.extend_from_slice(
        format!(
            "2 0 obj\n<< /Type /Pages /Kids [3 0 R{kids}] /Count {} >>\nendobj\n",
            pages + 1
        )
        .as_bytes(),
    );
    for index in 0..pages {
        offsets.push(bytes.len());
        let contents = if index < faulty { 4 } else { content };
        bytes.extend_from_slice(
            format!(
                "{} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << \
                 /Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint \
                 [0.9505 1.0 1.089] >>] >> >> /Contents {contents} 0 R >>\nendobj\n",
                first_new + index
            )
            .as_bytes(),
        );
    }
    let shown = "BT /F1 12 Tf 10 100 Td (Clean) Tj ET";
    offsets.push(bytes.len());
    bytes.extend_from_slice(
        format!(
            "{content} 0 obj\n<< /Length {} >>\nstream\n{shown}\nendstream\nendobj\n",
            shown.len()
        )
        .as_bytes(),
    );
    let xref = bytes.len();
    let mut table = format!(
        "xref\n2 1\n{tree:010} 00000 n \n{first_new} {}\n",
        pages + 1
    );
    for offset in offsets {
        let _ = writeln!(table, "{offset:010} 00000 n ");
    }
    bytes.extend_from_slice(
        format!(
            "{table}trailer\n<< /Size {} /Root 1 0 R /Prev {previous} /ID \
             [<12345678901234567890123456789012> <12345678901234567890123456789012>] >>\n\
             startxref\n{xref}\n%%EOF\n",
            content + 1
        )
        .as_bytes(),
    );
    bytes
}

#[test]
fn only_the_page_holding_the_fault_is_photographed_and_the_others_keep_their_text() {
    let source = with_more_pages(a_page_with_text_and_an_undefined_operator(), 0, 1);
    let target = Target::Two(Level::B);
    let (report, output) = convert_with_plan(&source, &printing("as-if-printed", target));
    let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    let pages = &conversion(&report).photographed;
    assert_eq!(pages.len(), 1, "{pages:?}");
    assert_eq!(pages[0].page, 1);
    let held = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
    let second = Pages::new(&held).get(1).expect("a second page");
    let resources = held.get_key(&second.dict, "Resources");
    assert!(
        held.resolve(&resources)
            .as_dict()
            .is_some_and(|resources| resources.get("Font").is_some()),
        "the second page keeps its fonts"
    );
}

#[test]
fn more_faulty_pages_than_the_bound_are_refused_by_name() {
    use pdf_transform::archive::MAX_PHOTOGRAPHED_PAGES;
    let target = Target::Two(Level::B);
    // The first page is faulty too, so `MAX_PHOTOGRAPHED_PAGES` more make one past the bound.
    let at_the_bound = with_more_pages(
        a_page_with_text_and_an_undefined_operator(),
        MAX_PHOTOGRAPHED_PAGES - 1,
        0,
    );
    let (report, output) = convert_with_plan(&at_the_bound, &printing("as-if-printed", target));
    assert!(output.is_some(), "{}", conversion(&report).render());
    assert_eq!(
        conversion(&report).photographed.len(),
        MAX_PHOTOGRAPHED_PAGES
    );
    let past = with_more_pages(
        a_page_with_text_and_an_undefined_operator(),
        MAX_PHOTOGRAPHED_PAGES,
        0,
    );
    let (report, output) = convert_with_plan(&past, &printing("as-if-printed", target));
    assert!(output.is_none(), "no file is written");
    let Decision::Refused(because) = decision(&report, "fonts/font-programs-embedded") else {
        panic!("{:?}", decision(&report, "fonts/font-programs-embedded"));
    };
    assert!(
        because.sentence().contains("conversion photographs"),
        "{}",
        because.sentence()
    );
}

#[test]
fn the_narrow_gate_photographs_more_pages_than_the_wider_gate_bound() {
    use pdf_transform::archive::MAX_PHOTOGRAPHED_PAGES;
    let target = Target::Two(Level::B);
    let source = with_more_pages(
        a_page_no_reader_can_read("", b"", "", &[]),
        MAX_PHOTOGRAPHED_PAGES,
        0,
    );
    let (report, output) = convert_with_plan(&source, &photographing(target));
    assert!(output.is_some(), "{}", conversion(&report).render());
    let pages = &conversion(&report).photographed;
    assert_eq!(pages.len(), MAX_PHOTOGRAPHED_PAGES + 1);
    assert!(pages.iter().all(|page| !page.anywhere), "{pages:?}");
}

/// A page showing `count` composite fonts, each of a character collection no route knows, which
/// every route refuses by name (§9.7.5.2: an identity `CMap` over a font the file does not embed).
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a fixture builder: object numbers a few dozen at most, no input is hostile"
)]
fn a_page_of_unknown_collections(count: usize) -> Vec<u8> {
    let mut fonts = String::new();
    let mut shown = Vec::new();
    let mut objects = Vec::new();
    for index in 0..count {
        let type0 = 9 + 2 * index;
        let _ = write!(fonts, "/G{index} {type0} 0 R ");
        shown.extend_from_slice(format!("BT /G{index} 12 Tf 10 100 Td <0024> Tj ET ").as_bytes());
        objects.push(format!(
            "<< /Type /Font /Subtype /Type0 /BaseFont /Unknown{index} /Encoding /Identity-H \
             /DescendantFonts [{} 0 R] >>",
            type0 + 1
        ));
        objects.push(
            "<< /Type /Font /Subtype /CIDFontType2 /BaseFont /Unknown /CIDSystemInfo \
             << /Registry (Custom) /Ordering (Collection) /Supplement 0 >> /FontDescriptor 8 0 R \
             /DW 1000 >>"
                .to_owned(),
        );
    }
    let objects: Vec<&str> = objects.iter().map(String::as_str).collect();
    a_page_no_reader_can_read(&fonts, &shown, "", &objects)
}

#[test]
fn fonts_photographed_around_are_bounded_and_the_next_keeps_its_refusal() {
    let target = Target::Two(Level::B);
    let (report, output) = convert_with_plan(
        &a_page_of_unknown_collections(16),
        &printing("as-if-printed", target),
    );
    assert!(output.is_some(), "{}", conversion(&report).render());
    assert_eq!(conversion(&report).photographed.len(), 1);
    let (report, output) = convert_with_plan(
        &a_page_of_unknown_collections(17),
        &printing("as-if-printed", target),
    );
    assert!(output.is_none(), "no file is written");
    assert!(matches!(
        decision(&report, "fonts/font-programs-embedded"),
        Decision::Refused(_)
    ));
}

// ---------------------------------------------------------------------------------------------
// A tagged document (A707, ADR 2516): the photographed page's leaves leave the structure tree
// ---------------------------------------------------------------------------------------------
//
// Expected values: ISO 32000-2 §14.7.5.2 (an integer in a structure element's `/K` is a
// marked-content identifier on the page the element's `/Pg` names, and a marked-content reference
// names the page its own `/Pg` names), §14.7.5.4 (a page's `/StructParents`), and A707 (under
// `as-if-printed`, at 2b and every part 4 flavour, the leaves that name a photographed page go and
// the rest of the tree stays, under `tagging-of-photographed-pages`, per page in the report and
// in the history; Level A keeps the refusal).

/// Two tagged pages: the first shows `Hello` and then an operator the base standard does not
/// define, the second `Kept`. One paragraph element names each page's first marked-content
/// sequence, and the second's also names the first page's second sequence through a
/// marked-content reference.
fn a_tagged_document_with_a_fault_on_its_first_page(target: Target) -> Vec<u8> {
    let kept = "/P <</MCID 0>> BDC BT /F1 12 Tf 10 100 Td (Kept) Tj ET EMC";
    let header = Conforming::part_two().header;
    Conforming {
        target,
        header: if matches!(target, Target::Two(_)) {
            header
        } else {
            None
        },
        catalog: "/MarkInfo << /Marked true >> /StructTreeRoot 9 0 R".to_owned(),
        page: "/StructParents 0".to_owned(),
        resources: "/Font << /F1 6 0 R >>".to_owned(),
        contents: Some((
            String::new(),
            b"/P <</MCID 0>> BDC BT /F1 12 Tf 10 100 Td (Hello) Tj ET EMC \
              /P <</MCID 1>> BDC 1 2 Frobnicate EMC"
                .to_vec(),
        )),
        objects: vec![
            HELVETICA.to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /Font << /F1 6 \
             0 R >> >> /Contents 8 0 R /StructParents 1 >>"
                .to_owned(),
            format!("<< /Length {} >>\nstream\n{kept}\nendstream", kept.len()),
            "<< /Type /StructTreeRoot /K 10 0 R /ParentTree 13 0 R /ParentTreeNextKey 2 >>"
                .to_owned(),
            "<< /Type /StructElem /S /Document /P 9 0 R /K [11 0 R 12 0 R] >>".to_owned(),
            "<< /Type /StructElem /S /P /P 10 0 R /Pg 3 0 R /K 0 >>".to_owned(),
            "<< /Type /StructElem /S /P /P 10 0 R /Pg 7 0 R /K [0 << /Type /MCR /Pg 3 0 R \
             /MCID 1 >>] >>"
                .to_owned(),
            "<< /Nums [0 [11 0 R 12 0 R] 1 [12 0 R]] >>".to_owned(),
        ],
        more_pages: vec![7],
        ..Conforming::default()
    }
    .build()
}

#[test]
fn a_photographed_page_of_a_tagged_document_leaves_the_structure_tree_and_the_rest_stays() {
    for target in [
        Target::Two(Level::B),
        Target::Four(pdf_archive::Flavour::Plain),
    ] {
        let source = a_tagged_document_with_a_fault_on_its_first_page(target);
        let (report, output) = convert_with_plan(&source, &printing("as-if-printed", target));
        let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
        assert_eq!(
            holds(&output, target).verdict(),
            Verdict::Conforms,
            "{target:?}"
        );
        if let Some(directory) = std::env::var_os("TAGGED_PHOTOGRAPH_ARTIFACTS") {
            let path = std::path::Path::new(&directory).join(format!("tagged-{target:?}.pdf"));
            std::fs::write(path, &output).expect("the artifact is written");
        }
        let Decision::Authorised { losses, .. } = decision(&report, OPERATORS) else {
            panic!("{:?}", decision(&report, OPERATORS));
        };
        assert!(
            losses.iter().eq([
                Loss::TextAsImage,
                Loss::PageAsImage,
                Loss::TaggingOfPhotographedPages
            ]),
            "{}",
            losses.words()
        );
        // The first page's two leaves went: the integer of its own paragraph and the second
        // paragraph's marked-content reference.
        let pages = &conversion(&report).photographed;
        assert_eq!(pages.len(), 1, "{pages:?}");
        assert_eq!((pages[0].page, pages[0].structure_leaves), (1, 2));
        let json = conversion(&report).to_json().render();
        assert!(json.contains("\"structure_leaves\": 2"), "{json}");
        assert!(json.contains("tagging-of-photographed-pages"), "{json}");
        let packet = String::from_utf8(super::document_packet(&output)).expect("UTF-8");
        assert!(
            packet.contains("tagging-of-photographed-pages")
                && packet.contains("2 leaves of the structure tree"),
            "{packet}"
        );
        // The tree is kept, and only the second page's leaf is in it; the photographed page no
        // longer states the parent-tree key of content it does not hold.
        let held = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
        let catalog = held.catalog().expect("a catalog");
        let root = held.resolve(&held.get_key(&catalog, "StructTreeRoot"));
        let root = root.as_dict().expect("the structure tree is kept");
        let document_element = held.resolve(&held.get_key(root, "K"));
        let document_element = document_element.as_dict().expect("an element");
        let Object::Array(paragraphs) = held.get_key(document_element, "K") else {
            panic!("two paragraphs");
        };
        assert_eq!(paragraphs.len(), 2, "both elements are kept");
        let first = held.resolve(&paragraphs[0]);
        assert!(first.as_dict().expect("an element").get("K").is_none());
        let second = held.resolve(&paragraphs[1]);
        assert_eq!(
            held.get_key(second.as_dict().expect("an element"), "K"),
            Object::Array(vec![Object::Integer(0)])
        );
        let every = Pages::new(&held).every();
        let page = |index: usize| every[index].as_ref().expect("a page").dict.clone();
        assert!(page(0).get("StructParents").is_none());
        assert_eq!(page(1).get("StructParents"), Some(&Object::Integer(1)));
    }
}

#[test]
fn a_tagged_document_keeps_the_refusal_where_the_tagging_loss_was_not_authorised() {
    let target = Target::Two(Level::B);
    let source = a_tagged_document_with_a_fault_on_its_first_page(target);
    let mut plan = printing("as-if-printed", target);
    plan.authorised.tagging_of_photographed_pages = false;
    let (report, output) = convert_with_plan(&source, &plan);
    assert!(output.is_none(), "no file is written");
    assert!(conversion(&report).photographed.is_empty());
    // Level A exists to keep the tagging, and is offered no photograph at all.
    let level_a = Target::Two(Level::A);
    let source = a_tagged_document_with_a_fault_on_its_first_page(level_a);
    let (report, output) = convert_with_plan(&source, &printing("as-if-printed", level_a));
    assert!(output.is_none(), "no file is written at Level A");
    assert!(conversion(&report).photographed.is_empty());
}

// ---------------------------------------------------------------------------------------------
// A document with optional content (A717): the photograph bakes in the configuration shown
// ---------------------------------------------------------------------------------------------
//
// Expected values: ISO 32000-2 §8.11.4.5 (on opening, `/BaseState` applies to every group and the
// `/OFF` array then adjusts the groups it names: "[t]his state shall be the initial state used by
// all PDF processors"), §8.11.3.2 (content between `BDC /OC` and `EMC` belongs to the group its
// property list names, through the page's `/Properties`), Table 96 (a group's `/Name`), and A717
// (under `as-if-printed`, at 2b and every part 4 flavour, a photographed page of such a document is
// drawn in the configuration this viewer shows, under `optional-content-of-photographed-pages`, the
// groups it fixed named per page in the report and in the history; the other pages keep their
// optional content; Level A and Level U keep the refusal).

/// Two pages and two optional content groups, `Shown` on and `Hidden` off by `/D`'s `/OFF`. The
/// first page shows `Hello`, a blue square in `Shown`, a red square in `Hidden`, and then an
/// operator the base standard does not define; the second draws only the blue square in `Shown`.
fn a_layered_document_with_a_fault_on_its_first_page(target: Target) -> Vec<u8> {
    let kept = "/OC /L1 BDC 0 0 1 rg 10 10 50 50 re f EMC";
    let header = Conforming::part_two().header;
    Conforming {
        target,
        header: if matches!(target, Target::Two(_)) {
            header
        } else {
            None
        },
        catalog: "/OCProperties << /OCGs [9 0 R 10 0 R] /D << /Name (Default) /Order [9 0 R 10 \
                  0 R] /OFF [10 0 R] >> >>"
            .to_owned(),
        resources: "/Font << /F1 6 0 R >> /Properties << /L1 9 0 R /L2 10 0 R >>".to_owned(),
        contents: Some((
            String::new(),
            b"BT /F1 12 Tf 10 100 Td (Hello) Tj ET \
              /OC /L1 BDC 0 0 1 rg 10 10 50 50 re f EMC \
              /OC /L2 BDC 1 0 0 rg 100 10 50 50 re f EMC \
              1 2 Frobnicate"
                .to_vec(),
        )),
        objects: vec![
            HELVETICA.to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << /Properties << \
             /L1 9 0 R >> >> /Contents 8 0 R >>"
                .to_owned(),
            format!("<< /Length {} >>\nstream\n{kept}\nendstream", kept.len()),
            "<< /Type /OCG /Name (Shown) >>".to_owned(),
            "<< /Type /OCG /Name (Hidden) >>".to_owned(),
        ],
        more_pages: vec![7],
        ..Conforming::default()
    }
    .build()
}

/// How many pixels of a raster are mostly red, and how many mostly blue.
fn red_and_blue(raster: &pdf_render::Raster) -> (usize, usize) {
    let mut counted = (0_usize, 0_usize);
    for pixel in raster.data.chunks_exact(4) {
        let [red, green, blue, _] = [pixel[0], pixel[1], pixel[2], pixel[3]];
        if red > 200 && green < 60 && blue < 60 {
            counted.0 = counted.0.saturating_add(1);
        }
        if blue > 200 && red < 60 && green < 60 {
            counted.1 = counted.1.saturating_add(1);
        }
    }
    counted
}

#[test]
fn a_photographed_page_of_a_layered_document_is_drawn_as_shown_and_names_the_groups_it_fixed() {
    for target in [
        Target::Two(Level::B),
        Target::Four(pdf_archive::Flavour::Plain),
    ] {
        let source = a_layered_document_with_a_fault_on_its_first_page(target);
        let (report, output) = convert_with_plan(&source, &printing("as-if-printed", target));
        let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
        assert_eq!(
            holds(&output, target).verdict(),
            Verdict::Conforms,
            "{target:?}"
        );
        if let Some(directory) = std::env::var_os("LAYERED_PHOTOGRAPH_ARTIFACTS") {
            let path = std::path::Path::new(&directory).join(format!("layered-{target:?}.pdf"));
            std::fs::write(path, &output).expect("the artifact is written");
        }
        let Decision::Authorised { losses, .. } = decision(&report, OPERATORS) else {
            panic!("{:?}", decision(&report, OPERATORS));
        };
        assert!(
            losses.iter().eq([
                Loss::TextAsImage,
                Loss::PageAsImage,
                Loss::OptionalContentOfPhotographedPages
            ]),
            "{}",
            losses.words()
        );
        let pages = &conversion(&report).photographed;
        assert_eq!(pages.len(), 1, "{pages:?}");
        let fixed = pages[0].optional_content.as_ref().expect("groups recorded");
        let named: Vec<(&str, Option<bool>)> = fixed
            .groups
            .iter()
            .map(|group| (group.name.as_str(), group.drawn))
            .collect();
        assert_eq!(named, [("Shown", Some(true)), ("Hidden", Some(false))]);
        let json = conversion(&report).to_json().render();
        assert!(
            json.contains("optional-content-of-photographed-pages")
                && json.contains("\"drawn\": \"off\""),
            "{json}"
        );
        assert!(
            conversion(&report).render().contains("\"Hidden\" off"),
            "{}",
            conversion(&report).render()
        );
        let packet = String::from_utf8(super::document_packet(&output)).expect("UTF-8");
        assert!(
            packet.contains("optional-content-of-photographed-pages")
                && (packet.contains("&quot;Hidden&quot; off") || packet.contains("\"Hidden\" off")),
            "{packet}"
        );
        // The photograph is the configuration shown: the hidden square is not in it, the shown
        // one is, and the page draws as the source's page did.
        let photographed = drawn(&output);
        let (red, blue) = red_and_blue(&photographed);
        assert_eq!(
            red, 0,
            "the group /D turns off is not drawn into the photograph"
        );
        assert!(blue > 0, "the group /D leaves on is");
        assert!(largest_difference(&drawn(&source), &photographed) <= 1);
        // The other page and the document keep their optional content.
        let held = Document::open_with_limits(output, Limits::DEFAULT).expect("it opens");
        let catalog = held.catalog().expect("a catalog");
        assert!(catalog.get("OCProperties").is_some());
        let every = Pages::new(&held).every();
        let second = every[1].as_ref().expect("a second page");
        assert!(
            held.get_key(&second.resources, "Properties")
                .as_dict()
                .is_some()
        );
    }
}

#[test]
fn a_layered_document_keeps_the_refusal_where_the_optional_content_loss_was_not_authorised() {
    let target = Target::Two(Level::B);
    let source = a_layered_document_with_a_fault_on_its_first_page(target);
    let mut plan = printing("as-if-printed", target);
    plan.authorised.optional_content_of_photographed_pages = false;
    let (report, output) = convert_with_plan(&source, &plan);
    assert!(output.is_none(), "no file is written");
    assert!(conversion(&report).photographed.is_empty());
    let rendered = conversion(&report).render();
    assert!(
        rendered.contains("states optional content")
            && rendered.contains("authorise optional-content-of-photographed-pages"),
        "{rendered}"
    );
    // Level A and Level U are offered no photograph at all.
    for target in [Target::Two(Level::A), Target::Two(Level::U)] {
        let source = a_layered_document_with_a_fault_on_its_first_page(target);
        let (report, output) = convert_with_plan(&source, &printing("as-if-printed", target));
        assert!(output.is_none(), "no file is written at {target:?}");
        assert!(conversion(&report).photographed.is_empty());
    }
}

#[test]
fn the_groups_a_photographed_page_names_are_listed_to_the_bound_and_the_rest_counted() {
    use pdf_transform::archive::{MAX_GROUP_NAME_CHARS, MAX_GROUPS_NAMED};
    let target = Target::Two(Level::B);
    let count = MAX_GROUPS_NAMED + 2;
    // Objects 7 onwards are the groups; the first states a name longer than the bound.
    let numbers: Vec<usize> = (7..7 + count).collect();
    let references: Vec<String> = numbers.iter().map(|n| format!("{n} 0 R")).collect();
    let properties: Vec<String> = numbers.iter().map(|n| format!("/G{n} {n} 0 R")).collect();
    let mut objects = vec![HELVETICA.to_owned()];
    objects.push(format!(
        "<< /Type /OCG /Name ({}) >>",
        "L".repeat(MAX_GROUP_NAME_CHARS + 1)
    ));
    objects.extend((1..count).map(|n| format!("<< /Type /OCG /Name (Group {n}) >>")));
    let source = Conforming {
        target,
        header: Conforming::part_two().header,
        catalog: format!(
            "/OCProperties << /OCGs [{0}] /D << /Name (Default) /Order [{0}] >> >>",
            references.join(" ")
        ),
        resources: format!(
            "/Font << /F1 6 0 R >> /Properties << {} >>",
            properties.join(" ")
        ),
        contents: Some((
            String::new(),
            b"BT /F1 12 Tf 10 100 Td (Hello) Tj ET /OC /G7 BDC 0 0 1 rg 10 10 50 50 re f EMC \
              1 2 Frobnicate"
                .to_vec(),
        )),
        objects,
        ..Conforming::default()
    }
    .build();
    let (report, output) = convert_with_plan(&source, &printing("as-if-printed", target));
    assert!(output.is_some(), "{}", conversion(&report).render());
    let pages = &conversion(&report).photographed;
    let fixed = pages[0].optional_content.as_ref().expect("groups recorded");
    assert_eq!((fixed.groups.len(), fixed.left_out), (MAX_GROUPS_NAMED, 2));
    let first = &fixed.groups[0].name;
    assert_eq!(
        first.chars().filter(|c| *c == 'L').count(),
        MAX_GROUP_NAME_CHARS,
        "{first}"
    );
    assert!(first.ends_with(" ..."), "{first}");
    assert!(fixed.named().ends_with(" and 2 more"), "{}", fixed.named());
}
