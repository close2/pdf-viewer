//! Closed reciprocal popup pairs require independent hidden-object answers.
//!
//! The expected values come from four places, named on each test: ISO 19005-2 section 6.3.2 and ISO
//! 19005-4 section 6.3.2 (every annotation but a `Popup` sets Print and clears Hidden, Invisible,
//! `ToggleNoView` and `NoView`), ISO 32000-2 §12.5.3's Table 167 (the flag values: Invisible 1,
//! Hidden 2, Print 4, `NoZoom` 8, `NoRotate` 16, `NoView` 32, `ToggleNoView` 256), Table 172's
//! markup `/Popup` and `/IRT` and Table 186's popup `/Parent` and `/Open` (§§12.5.6.2, 12.5.6.14),
//! and the fixture's own policy, `SHAPES`, which discards a hidden annotation and keeps a visible,
//! nonprinting one. `/F 30` is Hidden, Print, `NoZoom` and `NoRotate`.
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

/// A Text annotation and its popup, both `/F 30` (Hidden), leave the page together and the
/// output conforms: section 6.3.2 meets no hidden annotation in it, and Table 172's `/Popup` and
/// Table 186's `/Parent` name only each other. Two objects are removed, 6 and 7, with the
/// flags they stated, and the Square stays. An object outside the page that names the parent
/// keeps both, flags unchanged, because removing an annotation from `/Annots` is not deleting
/// the object; and the output converts again.
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

/// Every popup or parent that is not a closed, hidden, sole-linked member is refused rather than
/// guessed at: by Table 167, `/F 28`, an absent or null `/F`, and `/F 0` are not hidden, `/F 31`
/// sets Invisible, `/F 286` sets `ToggleNoView`, and `/F -1` read as bits sets every flag and
/// every bit §12.5.3 says "shall be set to 0"; Table 186's `/Open` is a boolean defaulting to
/// false, so `true` is open and `0` is not its type; a direct `/Parent` breaks Table 186's
/// indirect reference, and a dangling one or none at all leaves no parent to pair with; an
/// appearance, an `/AA` or a `/Popup` of the popup's own is a relationship the proof does not
/// infer. A parent stating `/Open` likewise, and a popup the page does not hold or holds
/// twice.
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

/// A third annotation naming the popup through Table 172's `/Popup` or `/IRT` or Table 186's
/// `/Parent` would be left naming an object no page holds, so each of the three is refused.
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

/// Removing the pair moves no pixel: Table 167's Hidden (`/F 34` is Hidden and `NoView`) says
/// "do not render the annotation", and the popup states no `/Open`, which Table 186 defaults
/// to false (closed). The page before and after is the same raster, and the one mark on it is
/// the Square's appearance stream, `0 0 1 rg`, so full blue [0, 0, 255, 255] by §8.6.4.3.
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

/// A second page referencing the same pair breaks §12.5.2's "\[a] given annotation dictionary
/// shall be referenced from the Annots array of only one page", and the proof refuses it rather
/// than choose a page.
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

/// Two corpus pairs, their object numbers and flags read from the files' own bytes: under the
/// shipped `as-shown` profile, `ZapfDingbats.pdf`'s `/F 30` pair leaves the site's decision
/// configured, and `OrphanPopups`' `/F 28` companion is Print, `NoZoom` and `NoRotate` by
/// Table 167, not hidden, so its conversion is refused.
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

/// A policy that discards visible shapes too does not make a visible popup (`/F 0`) a member of
/// a hidden pair: the proof needs both members hidden by Table 167, so it refuses.
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
