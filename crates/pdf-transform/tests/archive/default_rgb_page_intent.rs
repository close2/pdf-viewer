//! Page-level destination profiles and the explicit source default they cannot supply.
use super::*;
use pdf_syntax::Object;

fn named() -> Vec<u8> {
    std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"),
        "/../../doc/veraPDF-corpus/PDF_A-4/6.2 Graphics/6.2.4 Colour spaces/6.2.4.3 Uncalibrated -Device colour spaces/veraPDF test suite 6-2-4-3-t04-fail-n.pdf"))
        .expect("the held veraPDF witness")
}

fn object(document: &Document, number: u32) -> Object {
    document.get(ObjectId::new(number, 0))
}

fn raster(bytes: &[u8]) -> pdf_render::Raster {
    use pdf_render::Rasterizer as _;
    let document = Document::open(bytes.to_vec()).expect("document");
    let page = pdf_model::Pages::new(&document).get(0).expect("page");
    let drawn = pdf_model::interpret(&document, &page);
    assert!(drawn.unsupported.is_empty(), "{:?}", drawn.unsupported);
    let target =
        pdf_render::TargetSpec::for_page(&drawn.display_list, 2.0, 32 << 20).expect("target");
    render_cpu::CpuRasterizer::new()
        .rasterize(&drawn.display_list, target)
        .expect("raster")
}

#[test]
fn named_source_has_page_cmyk_intent_and_independent_gray_pixels() {
    let bytes = named();
    let document = Document::open(bytes.clone()).expect("source");
    assert!(
        document
            .catalog()
            .expect("catalog")
            .get("OutputIntents")
            .is_none()
    );
    let page = pdf_model::Pages::new(&document).get(0).expect("page");
    assert_eq!(page.id, Some(ObjectId::new(6, 0)));
    assert_eq!(
        page.dict.get("Group").and_then(Object::as_reference),
        Some(ObjectId::new(10, 0))
    );
    let intent = object(&document, 11);
    assert_eq!(
        intent
            .as_dict()
            .expect("intent")
            .get("DestOutputProfile")
            .and_then(Object::as_reference),
        Some(ObjectId::new(16, 0))
    );
    for (id, family, n) in [(15, b"GRAY", 1), (16, b"CMYK", 4)] {
        let held = object(&document, id);
        let stream = held.as_stream().expect("profile");
        assert_eq!(stream.dict.get("N").and_then(Object::as_integer), Some(n));
        let profile = document.decoded_stream_data(stream).expect("profile bytes");
        assert_eq!(profile.get(16..20), Some(family.as_slice()));
    }
    let form = object(&document, 12);
    let form = form.as_stream().expect("form");
    assert!(
        document
            .get_key(&form.dict, "Resources")
            .as_dict()
            .expect("own resources")
            .iter()
            .next()
            .is_none()
    );
    let content = document.decoded_stream_data(form).expect("form data");
    assert!(String::from_utf8_lossy(&content).contains("0.6875 0.765625 0.8671875 rg"));
    let image = raster(&bytes);
    assert_eq!((image.width, image.height), (1224, 1584));
    // §10.4.2.2 weights on the literal components, under the existing GreyRoute
    // inverse/forward convention: .3*11/16 + .59*49/64 + .11*111/128,
    // multiplied by255, is192.106640625. Points lie strictly inside the paths.
    for (x, y, expected) in [
        (45_usize, 660_usize, 192_u8),
        (100, 675, 192),
        (75, 685, 255),
        (20, 20, 255),
    ] {
        let at = ((792 - y) * 2 * 1224 + x * 2) * 4;
        assert_eq!(
            &image.data[at..at + 4],
            &[expected, expected, expected, 255],
            "PDF point({x},{y})"
        );
    }
}

#[test]
fn page_override_gets_a_default_without_changing_marks_or_profiles() {
    let source = named();
    let (report, output) = to_part_four(&source);
    let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
    assert_eq!(
        holds(&output, Target::Four(Flavour::Plain)).verdict(),
        Verdict::Conforms
    );
    let before = Document::open(source.clone()).expect("source");
    let after = Document::open(output.clone()).expect("output");
    let first = pdf_model::Pages::new(&before).get(0).expect("source page");
    let last = pdf_model::Pages::new(&after).get(0).expect("output page");
    assert_eq!(first.content(&before), last.content(&after));
    let old_form = before.get_key(&first.resources, "XObject");
    let old_form = before.get_key(old_form.as_dict().expect("XObjects"), "X0");
    let new_form = after.get_key(&last.resources, "XObject");
    let new_form = after.get_key(new_form.as_dict().expect("XObjects"), "X0");
    let old_form = old_form.as_stream().expect("old Form");
    let new_form = new_form.as_stream().expect("new Form");
    assert_eq!(
        before.decoded_stream_data(old_form),
        after.decoded_stream_data(new_form)
    );
    assert_eq!(old_form.dict.get("BBox"), new_form.dict.get("BBox"));
    let resources = after.get_key(&new_form.dict, "Resources");
    let spaces = after.get_key(resources.as_dict().expect("Form resources"), "ColorSpace");
    let default = after.get_key(spaces.as_dict().expect("spaces"), "DefaultRGB");
    is_icc_rgb(&after, default.as_array().expect("explicit Form default"));
    assert_eq!(page_profiles(&before), page_profiles(&after));
    let source_image = raster(&source);
    let image = raster(&output);
    assert_eq!(
        (source_image.width, source_image.height),
        (image.width, image.height)
    );
    assert_eq!(source_image.data.len(), image.data.len());
    let maximum = source_image
        .data
        .iter()
        .zip(&image.data)
        .map(|(a, b)| a.abs_diff(*b))
        .max()
        .expect("pixels");
    assert!(
        maximum <= 1,
        "the predeclared sRGB roundtrip allowance is one level, got {maximum}"
    );
    let (again, repeat) = to_part_four(&output);
    let repeat = repeat.unwrap_or_else(|| panic!("{}", conversion(&again).render()));
    assert!(!has_default_decision(&again));
    assert_eq!(
        page_profiles(&Document::open(repeat.clone()).expect("repeat")),
        page_profiles(&after)
    );
    assert_eq!(raster(&repeat).data, image.data);
    if let Some(directory) = std::env::var_os("DEFAULT_RGB_ARTIFACTS") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).expect("artifacts");
        for (name, bytes) in [("source.pdf", &source), ("output.pdf", &output)] {
            std::fs::write(directory.join(name), bytes).expect("PDF");
        }
        for (name, image) in [("source.png", source_image), ("output.png", image)] {
            let file = std::fs::File::create(directory.join(name)).expect("PNG");
            let mut encoder = png::Encoder::new(file, image.width, image.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .expect("header")
                .write_image_data(&image.data)
                .expect("pixels");
        }
    }
}

fn page_profiles(document: &Document) -> (Vec<u8>, Vec<u8>) {
    let page = pdf_model::Pages::new(document).get(0).expect("page");
    let intents = document.get_key(&page.dict, "OutputIntents");
    let intent = document.resolve(&intents.as_array().expect("intents")[0]);
    let profile = document.get_key(intent.as_dict().expect("intent"), "DestOutputProfile");
    let profile = document
        .decoded_stream_data(profile.as_stream().expect("intent profile"))
        .expect("bytes");
    let group = document.get_key(&page.dict, "Group");
    let space = document.get_key(group.as_dict().expect("group"), "CS");
    let gray = document.resolve(&space.as_array().expect("group space")[1]);
    let gray = document
        .decoded_stream_data(gray.as_stream().expect("group profile"))
        .expect("bytes");
    (profile.to_vec(), gray.to_vec())
}

fn synthetic(page_extra: &str, resources_extra: &str, form_extra: &str, marks: &[u8]) -> Vec<u8> {
    specimen(page_extra, resources_extra, form_extra, marks).build()
}

fn specimen(page_extra: &str, resources_extra: &str, form_extra: &str, marks: &[u8]) -> Conforming {
    let (cmyk, gray) = page_profiles(&Document::open(named()).expect("source"));
    Conforming {
        page: format!(
            "/OutputIntents [<< /S /GTS_PDFA1 /DestOutputProfile 7 0 R >>] /Group << /S /Transparency /CS [/ICCBased 6 0 R] >> {page_extra}"
        ),
        resources: format!("/XObject << /X0 8 0 R >> {resources_extra}"),
        contents: Some((String::new(), b"q /X0 Do Q\n".to_vec())),
        binary_objects: vec![
            stream("/N 1", &gray),
            stream("/N 4", &cmyk),
            stream(
                &format!(
                    "/Type /XObject /Subtype /Form /BBox [0 0 100 100] /Resources << >> {form_extra}"
                ),
                marks,
            ),
        ],
        ..Conforming::default()
    }
}

#[test]
fn a_synthetic_gray_page_preserves_literal_device_rgb() {
    let source = synthetic("", "", "", b"0.25 0.5 0.75 rg 10 10 40 40 re f\n");
    let (report, output) = to_part_four(&source);
    let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
    assert_eq!(
        holds(&output, Target::Four(Flavour::Plain)).verdict(),
        Verdict::Conforms
    );
    let before = raster(&source);
    let after = raster(&output);
    assert_eq!(before.data.len(), after.data.len());
    assert!(
        before
            .data
            .iter()
            .zip(&after.data)
            .all(|(a, b)| a.abs_diff(*b) <= 1)
    );
}

#[test]
fn unproved_consumers_never_take_the_new_default_route() {
    let marks = b"0.25 0.5 0.75 rg 10 10 40 40 re f\n";
    for (label, page, resources, form, content) in [
        (
            "annotation",
            "/Annots [<< /Subtype /Text >>]",
            "",
            "",
            marks.as_slice(),
        ),
        (
            "graphics state",
            "",
            "/ExtGState << >>",
            "",
            marks.as_slice(),
        ),
        ("font", "", "/Font << >>", "", marks.as_slice()),
        ("pattern", "", "/Pattern << >>", "", marks.as_slice()),
        ("shading", "", "/Shading << >>", "", marks.as_slice()),
        ("properties", "", "/Properties << >>", "", marks.as_slice()),
        ("external Form", "", "", "/Ref << >>", marks.as_slice()),
        (
            "Form group",
            "",
            "",
            "/Group << /S /Transparency /CS /DeviceRGB >>",
            marks.as_slice(),
        ),
        (
            "intent operator",
            "",
            "",
            "",
            b"/AbsoluteColorimetric ri 1 0 0 rg 10 10 40 40 re f\n".as_slice(),
        ),
        (
            "named source",
            "",
            "",
            "",
            b"/DeviceRGB cs 1 0 0 sc 10 10 40 40 re f\n".as_slice(),
        ),
        (
            "inline image",
            "",
            "",
            "",
            b"1 0 0 rg 10 10 40 40 re f BI /W 1 /H 1 /BPC 8 /CS /RGB ID abc EI\n".as_slice(),
        ),
        (
            "undecodable Form",
            "",
            "",
            "/Filter /UnknownDecode",
            marks.as_slice(),
        ),
    ] {
        let source = synthetic(page, resources, form, content);
        let (report, output) = to_part_four(&source);
        assert!(
            output.is_none(),
            "{label}: {}",
            conversion(&report).render()
        );
        assert!(
            !conversion(&report).decided.iter().any(|row| matches!(
                row.decision,
                Decision::Stated {
                    rewrite: Rewrite::DefaultRgb,
                    ..
                }
            )),
            "{label}"
        );
    }
}

const MARKS: &[u8] = b"0.25 0.5 0.75 rg 10 10 40 40 re f\n";
const SRGB: &[u8] = include_bytes!("../../../../data/icc/sRGB2014.icc");

fn has_default_decision(report: &Report) -> bool {
    conversion(report).decided.iter().any(|row| {
        matches!(
            row.decision,
            Decision::Stated {
                rewrite: Rewrite::DefaultRgb,
                ..
            }
        )
    })
}

#[test]
fn page_context_and_profile_boundaries_keep_the_old_fallback() {
    for (label, changed) in [
        (
            "wrong page N",
            "/OutputIntents [<< /S /GTS_PDFA1 /DestOutputProfile 6 0 R >>] /Group << /S /Transparency /CS /DeviceRGB >>",
        ),
        (
            "dangling profile",
            "/OutputIntents [<< /S /GTS_PDFA1 /DestOutputProfile 99 0 R >>]",
        ),
        (
            "conflicting profiles",
            "/OutputIntents [<< /S /GTS_PDFA1 /DestOutputProfile 7 0 R >> << /S /Other /DestOutputProfile 6 0 R >>]",
        ),
        (
            "external profile",
            "/OutputIntents [<< /S /GTS_PDFA1 /DestOutputProfile 7 0 R /DestOutputProfileRef << >> >>]",
        ),
        (
            "RGB group",
            "/OutputIntents [<< /S /GTS_PDFA1 /DestOutputProfile 7 0 R >>] /Group << /S /Transparency /CS /DeviceRGB >>",
        ),
    ] {
        let mut fixture = specimen("", "", "", MARKS);
        fixture.page = changed.to_owned();
        let (report, output) = to_part_four(&fixture.build());
        assert!(!has_default_decision(&report), "{label}");
        assert!(
            output.is_none(),
            "{label}: {}",
            conversion(&report).render()
        );
    }
    let source = synthetic("", "", "", MARKS);
    let (report, output) =
        convert_with_profile(&source, Target::Four(Flavour::Plain), &SRGB.into());
    assert!(!has_default_decision(&report));
    assert!(output.is_none());
    let mut held = specimen("", "", "", MARKS);
    held.catalog = "/OutputIntents [<< /S /GTS_PDFA1 /DestOutputProfile 9 0 R >>]".to_owned();
    held.binary_objects.push(stream("/N 3", SRGB));
    let (report, output) = to_part_four(&held.build());
    assert!(!has_default_decision(&report));
    assert!(output.is_none());
}

#[test]
fn ordinary_rgb_and_part_two_do_not_enter_page_override_preparation() {
    let mut fixture = specimen("", "", "", MARKS);
    fixture.page = "/Group << /S /Transparency /CS [/ICCBased 6 0 R] >>".to_owned();
    let (report, output) = to_part_four(&fixture.build());
    assert!(output.is_some(), "{}", conversion(&report).render());
    assert!(!has_default_decision(&report));
    let (report, _) = convert(
        &synthetic("", "", "", MARKS),
        Target::Two(Level::B),
        Authorisations::default(),
    );
    assert!(!has_default_decision(&report));
}

/// An incremental fixture revision, so both pages really share the same Form
/// object and resource dictionary instead of merely having equal dictionaries.
fn shared_pages(second: &str, extra: Option<Vec<u8>>) -> Vec<u8> {
    let mut source = synthetic("", "", "", MARKS);
    let text = String::from_utf8_lossy(&source);
    let previous = text
        .rsplit_once("startxref\n")
        .expect("xref")
        .1
        .lines()
        .next()
        .expect("offset")
        .to_owned();
    let mut additions = vec![
        (
            2,
            b"<< /Type /Pages /Kids [3 0 R 9 0 R] /Count 2 /Resources 10 0 R >>".to_vec(),
        ),
        (
            9,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R {second} >>"
            )
            .into_bytes(),
        ),
        (10, b"<< /XObject << /X0 8 0 R >> >>".to_vec()),
    ];
    if let Some(extra) = extra {
        additions.push((11, extra));
    }
    let size = if additions.len() == 4 { 12 } else { 11 };
    additions.push((3, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources 10 0 R /Contents 4 0 R /OutputIntents [<< /S /GTS_PDFA1 /DestOutputProfile 7 0 R >>] /Group << /S /Transparency /CS [/ICCBased 6 0 R] >> >>".to_vec()));
    additions.sort_by_key(|(id, _)| *id);
    let mut offsets = Vec::new();
    for (id, body) in additions {
        offsets.push((id, source.len()));
        source.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
        source.extend_from_slice(&body);
        source.extend_from_slice(b"\nendobj\n");
    }
    let xref = source.len();
    source.extend_from_slice(b"xref\n");
    for (id, offset) in offsets {
        source.extend_from_slice(format!("{id} 1\n{offset:010} 00000 n \n").as_bytes());
    }
    source.extend_from_slice(format!("trailer\n<< /Size {size} /Root 1 0 R /Prev {previous} /ID [<1D3A4B5C6D7E8F90A1B2C3D4E5F60718> <1D3A4B5C6D7E8F90A1B2C3D4E5F60718>] >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    source
}

#[test]
fn all_pages_are_proved_before_a_shared_form_is_changed() {
    let source = shared_pages("", None);
    let (report, output) = to_part_four(&source);
    let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
    let document = Document::open(output).expect("output");
    let pages = pdf_model::Pages::new(&document).every();
    assert_eq!(pages.len(), 2);
    let forms: Vec<_> = pages
        .iter()
        .map(|page| {
            let objects = document.get_key(&page.as_ref().expect("page").resources, "XObject");
            objects
                .as_dict()
                .expect("objects")
                .get("X0")
                .and_then(Object::as_reference)
                .expect("shared Form")
        })
        .collect();
    assert_eq!(forms[0], forms[1]);
    assert_eq!(
        pages[0].as_ref().expect("first").dict.get("Resources"),
        pages[1].as_ref().expect("second").dict.get("Resources")
    );
    let mut other_rgb = SRGB.to_vec();
    other_rgb[84] ^= 1; // ICC profile-ID bytes differ; N and RGB header alone cannot license insertion.
    let source = shared_pages(
        "/OutputIntents [<< /S /GTS_PDFA1 /DestOutputProfile 11 0 R >>]",
        Some(stream("/N 3", &other_rgb)),
    );
    let (report, output) = to_part_four(&source);
    assert!(output.is_none());
    assert!(!has_default_decision(&report));
}

#[test]
fn unused_resource_contexts_and_existing_defaults_are_checked_before_global_placement() {
    for identical in [true, false] {
        let mut fixture = specimen("", "", "", MARKS);
        fixture.resources = "/XObject << /X0 8 0 R /X1 10 0 R >>".to_owned();
        let mut producer_profile = SRGB.to_vec();
        if !identical {
            producer_profile[84] ^= 1;
        }
        fixture
            .binary_objects
            .push(stream("/N 3", &producer_profile));
        fixture.binary_objects.push(stream("/Type /XObject /Subtype /Form /BBox [0 0 100 100] /Resources << /ColorSpace << /DefaultRGB [/ICCBased 9 0 R] >> >>", MARKS));
        let (report, output) = to_part_four(&fixture.build());
        if identical {
            let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
            let document = Document::open(output).expect("output");
            let page = pdf_model::Pages::new(&document).get(0).expect("page");
            let objects = document.get_key(&page.resources, "XObject");
            let form = document.get_key(objects.as_dict().expect("objects"), "X1");
            let resources = document.get_key(&form.as_stream().expect("Form").dict, "Resources");
            let spaces = document.get_key(resources.as_dict().expect("resources"), "ColorSpace");
            let default = document.get_key(spaces.as_dict().expect("spaces"), "DefaultRGB");
            let profile = document.resolve(&default.as_array().expect("default")[1]);
            assert_eq!(
                document
                    .decoded_stream_data(profile.as_stream().expect("profile"))
                    .expect("bytes")
                    .as_ref(),
                producer_profile
            );
        } else {
            assert!(output.is_none());
            assert!(!has_default_decision(&report));
        }
    }
}

#[test]
fn structural_cycles_dangling_resources_and_content_budget_refuse() {
    for (label, form_dict, marks) in [
        (
            "cycle",
            "/Resources << /XObject << /Again 8 0 R >> >>",
            MARKS.to_vec(),
        ),
        ("dangling", "/Resources 99 0 R", MARKS.to_vec()),
        (
            "budget",
            "/Resources << >>",
            [vec![b' '; 1_048_577], MARKS.to_vec()].concat(),
        ),
    ] {
        let mut fixture = specimen("", "", "", MARKS);
        fixture.binary_objects[2] = stream(
            &format!("/Type /XObject /Subtype /Form /BBox [0 0 100 100] {form_dict}"),
            &marks,
        );
        let (report, output) = to_part_four(&fixture.build());
        assert!(
            output.is_none(),
            "{label}: {}",
            conversion(&report).render()
        );
        assert!(!has_default_decision(&report), "{label}");
    }
}

/// Adds a two-knot XYZ lookup table to the original gray profile. Its unequal
/// PCS white coordinates describe a tinted gray axis; a channel-count check
/// alone must not mistake it for the neutral tone-curve convention.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "the held 408-byte gray fixture has a fixed small tag table; each added offset is checked by the parser below"
)]
fn tinted_gray_profile() -> Vec<u8> {
    let (_, gray) = page_profiles(&Document::open(named()).expect("source"));
    let word = |at: usize| u32::from_be_bytes(gray[at..at + 4].try_into().expect("word"));
    let count = usize::try_from(word(128)).expect("tags");
    let old_end = 132 + count * 12;
    let mut profile = gray[..old_end].to_vec();
    profile[128..132].copy_from_slice(&u32::try_from(count + 1).expect("tags").to_be_bytes());
    for index in 0..count {
        let at = 136 + index * 12;
        profile[at..at + 4]
            .copy_from_slice(&word(at).checked_add(12).expect("offset").to_be_bytes());
    }
    let mut table = b"mft2\0\0\0\0\x01\x03\x02\0".to_vec();
    for value in [65536_i32, 0, 0, 0, 65536, 0, 0, 0, 65536] {
        table.extend_from_slice(&value.to_be_bytes());
    }
    table.extend_from_slice(&[0, 2, 0, 2]);
    for value in [
        0_u16, 65535, 0, 0, 0, 32768, 32768, 32768, 0, 65535, 0, 65535, 0, 65535,
    ] {
        table.extend_from_slice(&value.to_be_bytes());
    }
    profile.extend_from_slice(b"A2B1");
    profile.extend_from_slice(
        &u32::try_from(gray.len() + 12)
            .expect("offset")
            .to_be_bytes(),
    );
    profile.extend_from_slice(&u32::try_from(table.len()).expect("length").to_be_bytes());
    profile.extend_from_slice(&gray[old_end..]);
    profile.extend_from_slice(&table);
    let size = u32::try_from(profile.len()).expect("length");
    profile[..4].copy_from_slice(&size.to_be_bytes());
    profile
}

#[test]
fn a_gray_lookup_table_is_outside_the_neutral_curve_proof() {
    let profile = tinted_gray_profile();
    let parsed = pdf_model::icc::Profile::parse(&profile).expect("a supported gray LUT");
    let space = pdf_model::colour::ColourSpace::Icc {
        profile: std::sync::Arc::new(parsed),
    };
    let route = pdf_model::colour::GreyRoute::of(&space).expect("a monotone gray route exists");
    assert!(
        route
            .curve()
            .samples()
            .iter()
            .any(|sample| (sample[0] - sample[1]).abs() > 0.01),
        "the fixture really has a tinted axis"
    );
    let mut fixture = specimen("", "", "", MARKS);
    fixture.binary_objects[0] = stream("/N 1", &profile);
    let (report, _) = to_part_four(&fixture.build());
    assert!(
        !has_default_decision(&report),
        "{}",
        conversion(&report).render()
    );
}
