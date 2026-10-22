//! Closed reciprocal popup pairs require independent hidden-object answers.
use super::{
    Conforming, Decision, Flavour, Level, Target, Verdict, conversion, convert_with_plan, decision,
    holds, plan_from, stream,
};
use pdf_syntax::{Document, Object};

const SITE: &str = "annotations/printable-and-visible";
const SHAPES: &str = r#"
[site."annotations/printable-and-visible"]
remedy = "stop"
[site."annotations/printable-and-visible".shape."hidden-at-rest"]
remedy = "discard"
[site."annotations/printable-and-visible".shape."visible-nonprinting"]
remedy = "preserve"
"#;

fn fixture(target: Target, parent: &str, popup: &str, annots: &str, catalog: &str) -> Vec<u8> {
    let base = if matches!(target, Target::Two(_)) {
        Conforming::part_two()
    } else {
        Conforming::default()
    };
    Conforming {
        catalog: catalog.to_owned(),
        page: format!("/Annots [{annots}]"),
        objects: vec![
            format!("<< /Type /Annot /Subtype /Text /Rect [0 0 20 20] /F 30 /AP << /N 9 0 R >> {parent} >>"),
            format!("<< /Type /Annot /Subtype /Popup /Rect [0 0 20 20] {popup} >>"),
            "<< /Type /Annot /Subtype /Square /Rect [30 0 50 20] /F 0 /AP << /N 9 0 R >> >>".to_owned(),
        ],
        binary_objects: vec![stream("/Type /XObject /Subtype /Form /BBox [0 0 20 20] /Resources << >>", b"0 0 1 rg 0 0 20 20 re f\n")],
        ..base
    }.build()
}

#[test]
fn closed_hidden_pairs_leave_together_without_erasing_external_references() {
    for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
        for annots in ["6 0 R 7 0 R 8 0 R", "8 0 R 7 0 R 6 0 R"] {
            for external in ["", "/PairEvidence << /Held 6 0 R >>"] {
                let source = fixture(
                    target,
                    "/Popup 7 0 R",
                    "/F 30 /Parent 6 0 R /Open false",
                    annots,
                    external,
                );
                let (report, output) = convert_with_plan(&source, &plan_from(SHAPES, target));
                let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
                assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
                let removed = &conversion(&report).removed_annotations;
                assert_eq!(removed.len(), 2);
                let ids: std::collections::BTreeSet<_> = removed.iter().map(|row| row.at).collect();
                assert_eq!(
                    ids,
                    [
                        pdf_syntax::ObjectId::new(6, 0),
                        pdf_syntax::ObjectId::new(7, 0)
                    ]
                    .into_iter()
                    .collect()
                );
                assert!(
                    removed
                        .iter()
                        .all(|row| row.flags == Some(30) && row.page == 0)
                );
                let document = Document::open(output.clone()).expect("output");
                let page = pdf_model::Pages::new(&document).get(0).expect("page");
                let annotations = document.get_key(&page.dict, "Annots");
                assert_eq!(annotations.as_array().expect("Annots").len(), 1);
                if !external.is_empty() {
                    let catalog = document.catalog().expect("catalog");
                    let evidence = document.get_key(&catalog, "PairEvidence");
                    let parent = document.get_key(evidence.as_dict().expect("evidence"), "Held");
                    let parent = parent.as_dict().expect("retained parent");
                    let popup = document.get_key(parent, "Popup");
                    let popup = popup.as_dict().expect("retained popup");
                    assert_eq!(
                        document.get_key(popup, "Parent"),
                        Object::Dictionary(parent.clone())
                    );
                    assert_eq!(document.get_key(parent, "F"), Object::Integer(30));
                    assert_eq!(document.get_key(popup, "F"), Object::Integer(30));
                }
                assert!(
                    convert_with_plan(&output, &plan_from(SHAPES, target))
                        .1
                        .is_some()
                );
            }
        }
    }
}

#[test]
fn linked_pair_proof_refuses_unconsented_or_ambiguous_companions() {
    let target = Target::Four(Flavour::Plain);
    for popup in [
        "/F 28 /Parent 6 0 R /Open false", // Print+NoZoom+NoRotate, NOT hidden.
        "/Parent 6 0 R",
        "/F null /Parent 6 0 R",
        "/F 0 /Parent 6 0 R",
        "/F 30 /Parent 6 0 R /Open true",
        "/F 30 /Parent 6 0 R /Open 0",
        "/F 30 /Parent 66 0 R",
        "/F 30 /Parent << >>",
        "/F 30",
        "/F 30 /Parent 6 0 R /AP << /N 9 0 R >>",
        "/F 30 /Parent 6 0 R /AA << >>",
        "/F 30 /Parent 6 0 R /Popup 6 0 R",
        "/F 31 /Parent 6 0 R",
        "/F -1 /Parent 6 0 R",
        "/F 286 /Parent 6 0 R",
    ] {
        let source = fixture(target, "/Popup 7 0 R", popup, "6 0 R 7 0 R 8 0 R", "");
        let (report, output) = convert_with_plan(&source, &plan_from(SHAPES, target));
        assert!(output.is_none(), "{popup}");
        assert!(
            matches!(decision(&report, SITE), Decision::Refused(_)),
            "{popup}: {}",
            conversion(&report).render()
        );
    }
    for open in ["true", "0"] {
        let source = fixture(
            target,
            &format!("/Popup 7 0 R /Open {open}"),
            "/F 30 /Parent 6 0 R /Open false",
            "6 0 R 7 0 R 8 0 R",
            "",
        );
        let (report, output) = convert_with_plan(&source, &plan_from(SHAPES, target));
        assert!(output.is_none(), "parent Open {open}");
        assert!(matches!(decision(&report, SITE), Decision::Refused(_)));
    }
    for annots in ["6 0 R 8 0 R", "6 0 R 7 0 R 7 0 R 8 0 R"] {
        let source = fixture(target, "/Popup 7 0 R", "/F 30 /Parent 6 0 R", annots, "");
        assert!(
            convert_with_plan(&source, &plan_from(SHAPES, target))
                .1
                .is_none()
        );
    }
}

#[test]
fn linked_pair_proof_refuses_other_incoming_page_relationships() {
    let target = Target::Four(Flavour::Plain);
    for key in ["Popup", "Parent", "IRT"] {
        let source = Conforming {
            page: "/Annots [6 0 R 7 0 R 8 0 R]".to_owned(),
            objects: vec![
                "<< /Type /Annot /Subtype /Text /Rect [0 0 20 20] /F 30 /Popup 7 0 R /AP << /N 9 0 R >> >>".to_owned(),
                "<< /Type /Annot /Subtype /Popup /Rect [0 0 20 20] /F 30 /Parent 6 0 R >>".to_owned(),
                format!("<< /Type /Annot /Subtype /Square /Rect [30 0 50 20] /F 4 /{key} 7 0 R /AP << /N 9 0 R >> >>"),
            ],
            binary_objects: vec![stream("/Type /XObject /Subtype /Form /BBox [0 0 20 20] /Resources << >>", b"0 0 1 rg 0 0 20 20 re f\n")],
            ..Conforming::default()
        }.build();
        let (report, output) = convert_with_plan(&source, &plan_from(SHAPES, target));
        assert!(output.is_none(), "{key}");
        assert!(matches!(decision(&report, SITE), Decision::Refused(_)));
    }
}

fn raster(bytes: &[u8]) -> pdf_render::Raster {
    use pdf_render::Rasterizer as _;
    let document = Document::open(bytes.to_vec()).expect("document");
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
fn linked_pair_removal_preserves_actual_page_marks_and_closed_default() {
    for (label, target) in [
        ("2b", Target::Two(Level::B)),
        ("4", Target::Four(Flavour::Plain)),
    ] {
        let source = fixture(
            target,
            "/Popup 7 0 R",
            "/F 34 /Parent 6 0 R",
            "6 0 R 7 0 R 8 0 R",
            "",
        );
        let (report, output) = convert_with_plan(&source, &plan_from(SHAPES, target));
        let output = output.unwrap_or_else(|| panic!("{}", conversion(&report).render()));
        let before = raster(&source);
        let after = raster(&output);
        assert_eq!((before.width, before.height), (after.width, after.height));
        assert_eq!(before.data, after.data);
        assert!(
            after
                .data
                .chunks_exact(4)
                .any(|pixel| pixel == [0, 0, 255, 255]),
            "visible blue mark is nonempty"
        );
        let document = Document::open(output.clone()).expect("output");
        let page = pdf_model::Pages::new(&document).get(0).expect("page");
        let annotations = document.get_key(&page.dict, "Annots");
        let kept = document.resolve(
            annotations
                .as_array()
                .expect("Annots")
                .first()
                .expect("kept"),
        );
        let appearance = document.get_key(kept.as_dict().expect("annotation"), "AP");
        let normal = document.get_key(appearance.as_dict().expect("AP"), "N");
        assert_eq!(
            document
                .decoded_stream_data(normal.as_stream().expect("stream"))
                .expect("data")
                .as_ref(),
            b"0 0 1 rg 0 0 20 20 re f\n"
        );
        if let Some(directory) = std::env::var_os("LINKED_POPUP_ARTIFACTS") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).expect("directory");
            std::fs::write(directory.join(format!("{label}-source.pdf")), source).expect("source");
            std::fs::write(directory.join(format!("{label}-output.pdf")), output).expect("output");
            let file = std::fs::File::create(directory.join(format!("{label}.png"))).expect("PNG");
            let mut encoder = png::Encoder::new(file, after.width, after.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .expect("header")
                .write_image_data(&after.data)
                .expect("pixels");
        }
    }
}

#[test]
fn linked_pair_members_cannot_be_shared_across_pages() {
    let target = Target::Four(Flavour::Plain);
    let mut source = fixture(
        target,
        "/Popup 7 0 R",
        "/F 30 /Parent 6 0 R",
        "6 0 R 7 0 R 8 0 R",
        "",
    );
    let previous = String::from_utf8_lossy(&source)
        .rsplit_once("startxref\n")
        .expect("xref")
        .1
        .lines()
        .next()
        .expect("offset")
        .to_owned();
    let tree = source.len();
    source
        .extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R 10 0 R] /Count 2 >>\nendobj\n");
    let page = source.len();
    source.extend_from_slice(b"10 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> /Contents 4 0 R /Annots [6 0 R 7 0 R] >>\nendobj\n");
    let xref = source.len();
    source.extend_from_slice(format!("xref\n2 1\n{tree:010} 00000 n \n10 1\n{page:010} 00000 n \ntrailer\n<< /Size 11 /Root 1 0 R /Prev {previous} /ID [<12345678901234567890123456789012> <12345678901234567890123456789012>] >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    let document = Document::open(source.clone()).expect("source");
    assert_eq!(pdf_model::Pages::new(&document).every().len(), 2);
    let (report, output) = convert_with_plan(&source, &plan_from(SHAPES, target));
    assert!(output.is_none());
    assert!(matches!(decision(&report, SITE), Decision::Refused(_)));
}

#[test]
#[ignore = "named corpus witnesses; run explicitly when the external corpora are present"]
#[expect(
    clippy::print_stdout,
    reason = "the explicit corpus diagnostic records independent conversion refusals"
)]
fn named_popup_witnesses_use_the_project_parser_without_expanding_consent() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (name, path, parent, popup, flags) in [
        (
            "ZapfDingbats",
            "doc/pdf.js/test/pdfs/ZapfDingbats.pdf",
            43,
            44,
            30,
        ),
        (
            "OrphanPopups",
            "doc/corpora/pdfbox/pdfbox/src/test/resources/input/merge/PDFBOX-6018-099267-p9-OrphanPopups.pdf",
            23,
            43,
            28,
        ),
    ] {
        let source = std::fs::read(root.join(path)).expect("witness");
        let document = Document::open(source.clone()).expect("document");
        let parent_id = pdf_syntax::ObjectId::new(parent, 0);
        let popup_id = pdf_syntax::ObjectId::new(popup, 0);
        let parent = document.get(parent_id);
        let popup = document.get(popup_id);
        assert_eq!(
            parent.as_dict().expect("parent").get("Popup"),
            Some(&Object::Reference(popup_id))
        );
        assert_eq!(
            popup.as_dict().expect("popup").get("Parent"),
            Some(&Object::Reference(parent_id))
        );
        assert_eq!(
            document.get_key(popup.as_dict().expect("popup"), "F"),
            Object::Integer(flags)
        );
        for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
            let (report, output) = convert_with_plan(
                &source,
                &plan_from(
                    include_str!("../../../../doc/profiles/as-shown.toml"),
                    target,
                ),
            );
            println!(
                "{name} {target:?}: output={} {}",
                output.is_some(),
                conversion(&report).render()
            );
            if name == "ZapfDingbats" {
                assert!(matches!(
                    decision(&report, SITE),
                    Decision::Configured { .. }
                ));
            }
            if name == "OrphanPopups" {
                assert!(
                    output.is_none(),
                    "F28 companion is not an independently hidden annotation"
                );
            }
        }
    }
}

#[test]
fn discarding_visible_shapes_does_not_grant_hidden_pair_consent() {
    let target = Target::Four(Flavour::Plain);
    let source = fixture(
        target,
        "/Popup 7 0 R",
        "/F 0 /Parent 6 0 R",
        "6 0 R 7 0 R 8 0 R",
        "",
    );
    let policy = SHAPES.replace("remedy = \"preserve\"", "remedy = \"discard\"");
    let (report, output) = convert_with_plan(&source, &plan_from(&policy, target));
    assert!(output.is_none());
    assert!(matches!(decision(&report, SITE), Decision::Refused(_)));
}
