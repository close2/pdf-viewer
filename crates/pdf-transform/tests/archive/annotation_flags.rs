//! Table 167's screen and paper flags are independent answers inside one document.
use super::{
    Conforming, Decision, Flavour, Level, Limits, Target, Verdict, an_annotation_stating,
    conversion, convert_with_plan, decision, holds, plan_from, stream,
};
use pdf_syntax::{Document, Object};
use pdf_transform::archive::{Configuration, RemedyKind, Rewrite};

const SITE: &str = "annotations/printable-and-visible";
const SHAPES: &str = r#"
[site."annotations/printable-and-visible"]
remedy = "stop"
[site."annotations/printable-and-visible".shape."hidden-at-rest"]
remedy = "discard"
[site."annotations/printable-and-visible".shape."visible-nonprinting"]
remedy = "preserve"
"#;

fn mixed(target: Target) -> Vec<u8> {
    let marks = b"0 0 1 rg 0 0 20 20 re f\n";
    let base = if matches!(target, Target::Two(_)) {
        Conforming::part_two()
    } else {
        Conforming::default()
    };
    Conforming {
        page: "/Annots [6 0 R 7 0 R 8 0 R 9 0 R]".to_owned(),
        objects: [(0, 0, 20), (2, 30, 50), (32, 60, 80), (4, 90, 110)].into_iter().map(|(flags, left, right)| format!(
            "<< /Type /Annot /Subtype /Square /Rect [{left} 20 {right} 40] /F {flags} /AP << /N 10 0 R >> >>")).collect(),
        binary_objects: vec![stream("/Type /XObject /Subtype /Form /BBox [0 0 20 20] /Resources << >>", marks)],
        ..base
    }.build()
}

#[test]
fn shapes_keep_visible_marks_and_remove_hidden_ones_in_the_same_document() {
    // PDF/A §6.3.2 and Table 167: 0 is visible but nonprinting, 2/32 hide, 4 already conforms.
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        let source = mixed(target);
        let plan = plan_from(SHAPES, target);
        let (report, output) = convert_with_plan(&source, &plan);
        let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
        assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
        assert!(matches!(
            decision(&report, SITE),
            Decision::Configured {
                rewrite: Rewrite::AnnotationFlagsByShape,
                ..
            }
        ));
        let converted = conversion(&report);
        assert_eq!(
            converted
                .removed_annotations
                .iter()
                .map(|row| row.flags)
                .collect::<Vec<_>>(),
            vec![Some(2), Some(32)]
        );
        assert_eq!(
            converted
                .shown_annotations
                .iter()
                .map(|row| (row.stated, row.written))
                .collect::<Vec<_>>(),
            vec![(0, 4)]
        );
        let document = Document::open_with_limits(output, Limits::DEFAULT).expect("output");
        let page = pdf_model::Pages::new(&document).get(0).expect("page");
        let annotations = document.get_key(&page.dict, "Annots");
        let annotations = annotations.as_array().expect("kept annotations");
        assert_eq!(annotations.len(), 2);
        for annotation in annotations {
            let annotation = document.resolve(annotation);
            let dict = annotation.as_dict().expect("annotation");
            assert_eq!(document.get_key(dict, "F"), Object::Integer(4));
            let appearance = document.get_key(dict, "AP");
            let normal = document.get_key(appearance.as_dict().expect("AP"), "N");
            assert_eq!(
                document
                    .decoded_stream_data(normal.as_stream().expect("normal"))
                    .expect("decoded")
                    .as_ref(),
                b"0 0 1 rg 0 0 20 20 re f\n"
            );
        }
    }
}

#[test]
fn shapes_refuse_ambiguous_flags_even_if_global_removal_is_authorised() {
    for flags in [1, 256, 258, -1] {
        let source = an_annotation_stating(&flags.to_string(), "<< /N 7 0 R >>");
        let mut plan = plan_from(SHAPES, Target::Four(Flavour::Plain));
        plan.authorised.hidden_annotation = true;
        let (report, output) = convert_with_plan(&source, &plan);
        assert!(output.is_none());
        assert!(matches!(decision(&report, SITE), Decision::Refused(_)));
    }
}

#[test]
fn shape_configuration_preserves_unqualified_policy_and_order() {
    let target = Target::Four(Flavour::Plain);
    let plain = Configuration::read(
        &format!("[site.\"{SITE}\"]\nremedy = \"discard\"\n"),
        target,
    )
    .expect("plain");
    assert!(plain.annotation_flags.is_none());
    assert!(plain.authorisations(target).hidden_annotation);
    let first = Configuration::read(SHAPES, target).expect("shapes");
    assert!(!first.authorisations(target).hidden_annotation);
    let reverse = SHAPES
        .split("[site.")
        .skip(1)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .fold(String::new(), |mut text, block| {
            text.push_str("[site.");
            text.push_str(block);
            text
        });
    assert_eq!(
        first.annotation_flags,
        Configuration::read(&reverse, target)
            .expect("reordered")
            .annotation_flags
    );
    let answers = first.annotation_flags.expect("per-object answers");
    assert_eq!(answers.hidden, RemedyKind::Discard);
    assert_eq!(answers.visible, RemedyKind::Preserve);
    let shipped = pdf_transform::profiles::Contribution::read(
        include_str!("../../../../doc/profiles/as-shown.toml"),
        target,
    )
    .expect("shipped");
    assert_eq!(shipped.annotation_flags, Some(answers));
}

#[test]
fn shape_configuration_rejects_unknown_shapes_and_unhandled_fallback_options() {
    let target = Target::Four(Flavour::Plain);
    for text in [
        SHAPES.replace("hidden-at-rest", "hidden-at-resst"),
        SHAPES.replace("remedy = \"discard\"", "remedy = \"derive\""),
        SHAPES.replace(
            "remedy = \"stop\"",
            "remedy = \"preserve\"\non-failure = \"discard\"",
        ),
        SHAPES.replace(
            "remedy = \"preserve\"",
            "remedy = \"preserve\"\nplacement = \"append\"",
        ),
    ] {
        assert!(
            Configuration::read(&text, target).is_err(),
            "must reject {text}"
        );
    }
}

fn with_second_page(mut bytes: Vec<u8>) -> Vec<u8> {
    // §7.5.6: an incremental page-tree replacement, retaining shared annotation object identities.
    let text = String::from_utf8_lossy(&bytes);
    let previous = text
        .rsplit_once("startxref\n")
        .expect("xref")
        .1
        .lines()
        .next()
        .expect("offset")
        .to_owned();
    let tree = bytes.len();
    bytes.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R 11 0 R] /Count 2 >>\nendobj\n");
    let page = bytes.len();
    bytes.extend_from_slice(b"11 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> /Contents 4 0 R /Annots [6 0 R 7 0 R 8 0 R 9 0 R] >>\nendobj\n");
    let xref = bytes.len();
    bytes.extend_from_slice(format!("xref\n2 1\n{tree:010} 00000 n \n11 1\n{page:010} 00000 n \ntrailer\n<< /Size 12 /Root 1 0 R /Prev {previous} /ID [<12345678901234567890123456789012> <12345678901234567890123456789012>] >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    bytes
}

#[test]
fn shared_annotations_keep_one_identity_and_report_each_page_occurrence() {
    let target = Target::Four(Flavour::Plain);
    let source = with_second_page(mixed(target));
    let (report, output) = convert_with_plan(&source, &plan_from(SHAPES, target));
    let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    assert_eq!(
        conversion(&report)
            .removed_annotations
            .iter()
            .map(|row| row.page)
            .collect::<Vec<_>>(),
        vec![0, 0, 1, 1]
    );
    let shown = &conversion(&report).shown_annotations;
    assert_eq!(shown.len(), 2);
    assert_eq!(shown[0].at, shown[1].at);
    assert_eq!((shown[0].page, shown[1].page), (0, 1));
    let document = Document::open_with_limits(output, Limits::DEFAULT).expect("output");
    let pages = pdf_model::Pages::new(&document);
    let first = pages.get(0).expect("first page");
    let second = pages.get(1).expect("second page");
    assert_eq!(
        document.get_key(&first.dict, "Annots"),
        document.get_key(&second.dict, "Annots")
    );
}

#[test]
fn linked_annotations_and_widgets_are_refused_before_mixed_writes() {
    let target = Target::Four(Flavour::Plain);
    for (subtype, linked) in [
        ("Square", "/Popup 8 0 R"),
        ("Square", "/Parent 8 0 R"),
        ("Widget", ""),
    ] {
        let source = Conforming {
            page: "/Annots [6 0 R]".to_owned(),
            objects: vec![format!("<< /Type /Annot /Subtype /{subtype} /Rect [20 30 60 70] /F 2 /AP << /N 7 0 R >> {linked} >>")],
            binary_objects: vec![stream("/Type /XObject /Subtype /Form /BBox [0 0 40 40]", b"0 0 40 40 re f")],
            ..Conforming::default()
        }.build();
        let (report, output) = convert_with_plan(&source, &plan_from(SHAPES, target));
        assert!(output.is_none());
        assert!(matches!(decision(&report, SITE), Decision::Refused(_)));
    }
}

fn raster(bytes: &[u8]) -> pdf_render::Raster {
    use pdf_render::Rasterizer as _;
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("document");
    let page = pdf_model::Pages::new(&document).get(0).expect("page");
    let drawn = pdf_model::interpret(&document, &page);
    assert!(drawn.unsupported.is_empty(), "{:?}", drawn.unsupported);
    let target =
        pdf_render::TargetSpec::for_page(&drawn.display_list, 2.0, 16 << 20).expect("target");
    render_cpu::CpuRasterizer::new()
        .rasterize(&drawn.display_list, target)
        .expect("raster")
}

#[test]
fn shown_profile_keeps_the_actual_page_at_rest() {
    for (label, target) in [
        ("2b", Target::Two(Level::B)),
        ("4", Target::Four(Flavour::Plain)),
    ] {
        let source = mixed(target);
        let shipped = include_str!("../../../../doc/profiles/as-shown.toml");
        let (report, output) = convert_with_plan(&source, &plan_from(shipped, target));
        let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
        let before = raster(&source);
        let after = raster(&output);
        assert_eq!((before.width, before.height), (after.width, after.height));
        assert_eq!(before.data, after.data);
        if let Some(directory) = std::env::var_os("ANNOTATION_FLAG_ARTIFACTS") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).expect("artifact directory");
            std::fs::write(directory.join(format!("{label}-source.pdf")), &source).expect("source");
            std::fs::write(directory.join(format!("{label}-output.pdf")), &output).expect("output");
            let file =
                std::fs::File::create(directory.join(format!("{label}.png"))).expect("picture");
            let mut encoder = png::Encoder::new(file, after.width, after.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .expect("header")
                .write_image_data(&after.data)
                .expect("picture data");
        }
    }
}

#[test]
fn shown_profile_distinguishes_absent_flags_from_malformed_values() {
    let target = Target::Four(Flavour::Plain);
    let shipped = include_str!("../../../../doc/profiles/as-shown.toml");
    for (entry, permitted) in [
        ("", true),
        ("/F null", true),
        ("/F true", false),
        ("/F /Unknown", false),
    ] {
        let source = Conforming {
            page: "/Annots [6 0 R]".to_owned(),
            objects: vec![format!(
                "<< /Type /Annot /Subtype /Link /Rect [0 0 10 10] {entry} >>"
            )],
            ..Conforming::default()
        }
        .build();
        let mut plan = plan_from(shipped, target);
        plan.authorised.hidden_annotation = true;
        let (report, output) = convert_with_plan(&source, &plan);
        assert_eq!(
            output.is_some(),
            permitted,
            "{entry}: {}",
            conversion(&report).render()
        );
        if let Some(output) = output {
            assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
            let document = Document::open_with_limits(output, Limits::DEFAULT).expect("output");
            let page = pdf_model::Pages::new(&document).get(0).expect("page");
            let annotations = document.get_key(&page.dict, "Annots");
            let annotation = document.resolve(
                annotations
                    .as_array()
                    .expect("annotations")
                    .first()
                    .expect("annotation"),
            );
            assert_eq!(
                document.get_key(annotation.as_dict().expect("annotation"), "F"),
                Object::Integer(4)
            );
        } else {
            assert!(matches!(
                decision(&report, "annotations/flags-entry-present"),
                Decision::Refused(_)
            ));
        }
        if entry.is_empty() {
            assert!(
                convert_with_plan(&source, &plan_from(SHAPES, target))
                    .1
                    .is_none(),
                "shape policy does not authorise the separate absent-entry site"
            );
        }
    }
}

#[test]
fn the_previous_unqualified_stop_does_not_convert_the_mixed_population() {
    let target = Target::Four(Flavour::Plain);
    let old_answer = format!("[site.\"{SITE}\"]\nremedy = \"stop\"\n");
    let (report, output) = convert_with_plan(&mixed(target), &plan_from(&old_answer, target));
    assert!(
        output.is_none(),
        "the pre-split policy must remain a refusal"
    );
    assert!(!matches!(
        decision(&report, SITE),
        Decision::Configured { .. }
    ));
    let partial = format!(
        "{old_answer}[site.\"{SITE}\".shape.\"visible-nonprinting\"]\nremedy = \"preserve\"\n"
    );
    assert!(
        convert_with_plan(&mixed(target), &plan_from(&partial, target))
            .1
            .is_none(),
        "answering visible marks alone must not authorise hidden-object removal"
    );
}
