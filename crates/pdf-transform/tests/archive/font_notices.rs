//! A550's redistribution conditions survive archive preparation and serialization.

use super::*;

/// The embedded program selected by the fixture's F1 dictionary.
fn program(bytes: &[u8]) -> (Document, std::sync::Arc<pdf_syntax::Stream>) {
    let (document, font) = described_font(bytes);
    let descriptor = document.get_key(&font, "FontDescriptor");
    let descriptor = descriptor.as_dict().expect("descriptor");
    let stream = ["FontFile2", "FontFile3"]
        .into_iter()
        .find_map(|key| {
            let object = document.get_key(descriptor, key);
            if let pdf_syntax::Object::Stream(stream) = object {
                Some(stream)
            } else {
                None
            }
        })
        .expect("embedded font");
    (document, stream)
}

#[test]
fn every_bundled_foxit_face_carries_its_verbatim_notice_on_its_program_stream() {
    // ISO 32000-2 §14.3.2 Tables 347–348 attach metadata to the actual information resource.
    // PDF/A-2 6.6.2.1 and PDF/A-4 6.7.2.1 also apply to these object-level XMP packets.
    for name in [
        "Times-Roman",
        "Times-Bold",
        "Times-Italic",
        "Times-BoldItalic",
        "Courier",
        "Courier-Bold",
        "Courier-Oblique",
        "Courier-BoldOblique",
        "Symbol",
        "ZapfDingbats",
    ] {
        for target in [Target::Two(Level::B), Target::Four(Flavour::Plain)] {
            let source = a_standard_font_by_name(name, "");
            let (report, output) = convert(&source, target, Authorisations::default());
            let output = written(&report, output);
            assert_eq!(
                holds(&output, target).verdict(),
                Verdict::Conforms,
                "{name}"
            );
            let (document, program) = program(&output);
            let reference = program
                .dict
                .get("Metadata")
                .and_then(pdf_syntax::Object::as_reference)
                .expect("indirect metadata on program");
            let metadata = document.get(reference);
            let metadata = metadata.as_stream().expect("metadata stream");
            assert_eq!(name_entry(&document, &metadata.dict, "Type"), "Metadata");
            assert_eq!(name_entry(&document, &metadata.dict, "Subtype"), "XML");
            let bytes = document.decoded_stream_data(metadata).expect("packet");
            let text = std::str::from_utf8(&bytes).expect("UTF-8");
            let exact = text
                .split_once("<rdf:li xml:lang=\"x-default\">")
                .expect("item")
                .1
                .split_once("</rdf:li>")
                .expect("end")
                .0;
            assert_eq!(
                exact,
                include_str!("../../../../data/standard-fonts/LICENSE_FOXIT")
            );
            let properties = pdf_model::xmp::Xmp::parse_detail(&bytes).expect("XMP");
            let terms = properties
                .iter()
                .find(|property| {
                    property.name.namespace == "http://ns.adobe.com/xap/1.0/rights/"
                        && property.name.local == "UsageTerms"
                })
                .expect("rights");
            let alternatives = terms.value.alternatives().expect("language alternative");
            assert_eq!(alternatives.len(), 1);
            assert_eq!(
                alternatives
                    .first()
                    .map(|(language, value)| (language.as_deref(), value.text())),
                Some((
                    Some("x-default"),
                    Some(include_str!("../../../../data/standard-fonts/LICENSE_FOXIT").trim())
                ))
            );
        }
    }
}

#[test]
fn only_metric_modified_liberation_programs_are_renamed_in_both_embedding_routes() {
    // A550 chooses the OFL condition-3 rename for Modified Versions, not an alteration of
    // unmodified embedding. Exact equality to the renamed program also holds glyph tables.
    let original = include_bytes!("../../../../data/standard-fonts/LiberationSans-Regular.ttf");
    for embedded in [false, true] {
        for width in ["667", "500"] {
            let source = a_font_stating(width, embedded);
            let (report, output) =
                convert(&source, Target::Two(Level::B), the_face_losses_authorised());
            let output = written(&report, output);
            let (document, stream) = program(&output);
            let bytes = document.decoded_stream_data(&stream).expect("program");
            if width == "667" {
                assert_eq!(
                    bytes.as_ref(),
                    original,
                    "unmodified program, embedded={embedded}"
                );
            } else {
                let renamed = pdf_font::modified_name::liberation(&bytes).expect("read identity");
                assert_eq!(renamed, bytes.as_ref(), "renaming already happened");
                assert_ne!(bytes.as_ref(), original);
            }
            assert_eq!(stated_widths(&source), stated_widths(&output));
        }
    }
}

/// A Foxit program with a descriptor, optionally already embedded and carrying object metadata.
fn foxit_with_descriptor(embedded: bool, metadata: Option<&str>) -> Vec<u8> {
    let (program, _) = pdf_font::standard::face(pdf_font::substitute::Request {
        family: pdf_font::substitute::Family::Serif,
        bold: false,
        italic: false,
        standard: true,
    });
    let font_file = if embedded { "/FontFile3 8 0 R" } else { "" };
    let metadata_key = if metadata.is_some() {
        "/Metadata 9 0 R"
    } else {
        ""
    };
    let mut binary_objects = vec![stream(&format!("/Subtype /Type1C {metadata_key}"), program)];
    if let Some(packet) = metadata {
        binary_objects.push(stream("/Type /Metadata /Subtype /XML", packet.as_bytes()));
    }
    Conforming {
        resources: "/Font << /F1 6 0 R >> /ColorSpace << /CS0 [/CalGray << /WhitePoint [0.9505 1.0 1.089] >>] >>".to_owned(),
        contents: Some((String::new(), b"/CS0 cs 0 sc BT /F1 12 Tf 10 100 Td (A) Tj ET".to_vec())),
        objects: vec![
            "<< /Type /Font /Subtype /Type1 /BaseFont /Times-Roman /FirstChar 65 /LastChar 65 /Widths [500] /FontDescriptor 7 0 R /Encoding /WinAnsiEncoding >>".to_owned(),
            format!("<< /Type /FontDescriptor /FontName /Times-Roman /Flags 32 /FontBBox [-543 -303 1300 980] /ItalicAngle 0 /Ascent 905 /Descent -212 /CapHeight 716 /StemV 80 {font_file} >>"),
        ], binary_objects, ..Conforming::part_two()
    }.build()
}

#[test]
fn both_metric_restatement_routes_attach_foxit_notice_and_preserve_other_object_metadata() {
    // A550 includes programs written after metric restatement, not only the unmodified shared
    // stream. Existing metadata belongs to the font and must survive the additional notice.
    let packet = "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description rdf:about=\"\" xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\"><xmp:CreatorTool>font producer</xmp:CreatorTool></rdf:Description></rdf:RDF></x:xmpmeta>";
    // The fixture's one width, 500 for an `A` the face draws at 722, has the viewer draw the
    // substituted face narrowed to it, so the restated route loses that fitting and says so: the
    // loss `substitute-stretch`, authorised here because the notice is what this test is about
    // (`doc/adr/2262`).
    let authorised = Authorisations {
        substitute_stretch: true,
        glyphs_from_to_unicode: true,
        text_as_image: true,
        ..Authorisations::default()
    };
    for embedded in [false, true] {
        let source = foxit_with_descriptor(embedded, embedded.then_some(packet));
        let (report, output) = convert(&source, Target::Two(Level::B), authorised);
        let output = written(&report, output);
        assert_eq!(
            holds(&output, Target::Two(Level::B)).verdict(),
            Verdict::Conforms
        );
        let (document, program) = program(&output);
        let metadata = document.get_key(&program.dict, "Metadata");
        let metadata = metadata.as_stream().expect("notice stream");
        let bytes = document.decoded_stream_data(metadata).expect("XMP");
        let text = std::str::from_utf8(&bytes).expect("UTF-8");
        assert!(text.contains(include_str!(
            "../../../../data/standard-fonts/LICENSE_FOXIT"
        )));
        let xmp = pdf_model::xmp::Xmp::parse(&bytes).expect("parsed");
        assert!(
            xmp.value("http://ns.adobe.com/xap/1.0/rights/", "UsageTerms")
                .is_some()
        );
        if embedded {
            assert_eq!(
                xmp.value("http://ns.adobe.com/xap/1.0/", "CreatorTool"),
                Some(&pdf_model::xmp::Value::Text("font producer".to_owned()))
            );
        }
    }
}

#[test]
fn an_existing_different_foxit_license_is_not_silently_replaced() {
    let packet = "<x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description rdf:about=\"\" xmlns:xmpRights=\"http://ns.adobe.com/xap/1.0/rights/\"><xmpRights:UsageTerms><rdf:Alt><rdf:li xml:lang=\"x-default\">producer terms</rdf:li></rdf:Alt></xmpRights:UsageTerms></rdf:Description></rdf:RDF></x:xmpmeta>";
    let source = foxit_with_descriptor(true, Some(packet));
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    assert!(output.is_none());
    let refused = decision(&report, "fonts/widths-agree-with-the-program");
    assert!(
        format!("{refused:?}").contains("usage terms"),
        "{refused:?}"
    );
}

#[test]
fn removing_a_liberation_cmap_for_a_cid_font_also_renames_the_modified_program() {
    // §9.9.1 removes cmap from the CID program. That modification still invokes A550's OFL
    // condition even where the shown advance already agrees and no metric is restated.
    let source = a_composite_font_stating("Identity-H", "/W [36 [667]]");
    let plan = a_supplied_font(Target::Two(Level::B), "LiberationSans", LIBERATION_SANS);
    let (report, output) = convert_with_plan(&source, &plan);
    let output = written(&report, output);
    assert_eq!(
        conversion(&report).substituted.first().expect("font").route,
        pdf_transform::archive::MetricRoute::FaceMetrics
    );
    let document = Document::open_with_limits(output, Limits::DEFAULT).expect("output");
    let descendants = document.get_key(&font_resource(&document), "DescendantFonts");
    let descendant = document.resolve(
        descendants
            .as_array()
            .and_then(|array| array.first())
            .expect("descendant"),
    );
    let descriptor = document.get_key(descendant.as_dict().expect("CIDFont"), "FontDescriptor");
    let program = document.get_key(descriptor.as_dict().expect("descriptor"), "FontFile2");
    let bytes = document
        .decoded_stream_data(program.as_stream().expect("program"))
        .expect("bytes");
    assert_eq!(
        pdf_font::modified_name::liberation(&bytes).expect("renamed"),
        bytes.as_ref()
    );
    assert!(
        !pdf_font::embedding::sfnt_tables(&bytes)
            .expect("sfnt")
            .carries(b"cmap")
    );
}

#[test]
fn supplemented_foxit_terms_with_unrelated_metadata_are_reused_exactly() {
    let source = foxit_with_descriptor(
        true,
        Some(
            "<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description rdf:about=\"\" xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\"><xmp:CreatorTool>font producer</xmp:CreatorTool></rdf:Description></rdf:RDF>",
        ),
    );
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    let output = written(&report, output);
    let (document, font) = program(&output);
    let metadata = document.get_key(&font.dict, "Metadata");
    let bytes = document
        .decoded_stream_data(metadata.as_stream().expect("metadata"))
        .expect("packet");
    let packet = std::str::from_utf8(&bytes).expect("UTF-8");
    // Reuse the supplemented packet on the original bundled program, so provenance matching
    // takes the attachment route again rather than skipping an already modified program.
    let source = foxit_with_descriptor(true, Some(packet));
    let (report, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    let output = written(&report, output);
    let (document, font) = program(&output);
    let metadata = document.get_key(&font.dict, "Metadata");
    assert_eq!(
        document
            .decoded_stream_data(metadata.as_stream().expect("metadata"))
            .expect("packet")
            .as_ref(),
        bytes.as_ref()
    );
    let changed = packet.replace("All rights reserved.", "Different rights.");
    assert_ne!(changed, packet);
    let source = foxit_with_descriptor(true, Some(&changed));
    let (_, output) = convert(&source, Target::Two(Level::B), Authorisations::default());
    assert!(output.is_none());
}
