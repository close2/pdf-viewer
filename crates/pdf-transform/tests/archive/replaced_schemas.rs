//! A packet section 6.6.2.1 replaces meets section 6.6.2.3.2 by the replacement.
//!
//! ISO 19005-2 section 6.6.2.3.2 requires every extension schema a metadata stream uses to be
//! described in that stream or in the catalog's. Section 6.6.2.1 requires the packet to be one
//! `rdf:RDF` element, and a packet that is not is replaced by one stating no property at all
//! (`doc/adr/1245`) — which uses no extension schema, so the replacement is what meets the
//! schema requirement at that stream, and the decision is the replacement's own rather than a
//! refusal that no container was prepared (`doc/adr/2442`).
//!
//! The two editors after the container meet the same shape the same way (`doc/adr/2455`): section
//! 6.6.4's amendment identifier is a property of the catalog's packet, and section 6.6.2.3.1's
//! structure fields are properties of a packet's value types, so a packet stating no property
//! states neither, and each row takes the replacement's decision too.
use super::{
    Authorisations, Decision, Level, Loss, Losses, Rewrite, Target, Verdict, a_document_carrying,
    convert, convert_with_plan, decision, holds, plan_from,
};

const SCHEMAS: &str = "metadata/extension-schemas-embedded";
const ONE_RDF: &str = "metadata/xmp-packets-state-one-rdf-element";

/// A packet of two `rdf:RDF` elements, the second stating a property in a schema nothing in the
/// file describes: section 6.6.2.1's fault and section 6.6.2.3.2's in one stream.
fn two_rdf_elements_one_using_an_undescribed_schema() -> Vec<u8> {
    let mut packet = String::new();
    packet.push_str("<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n");
    packet.push_str("<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n");
    for property in [
        "xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\">\n<pdf:Producer>an exporter</pdf:Producer>\n",
        "xmlns:acme=\"http://acme.example/ns/1.0/\">\n<acme:BatchNumber>4417</acme:BatchNumber>\n",
    ] {
        packet.push_str("<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n");
        packet.push_str("<rdf:Description rdf:about=\"\" ");
        packet.push_str(property);
        packet.push_str("</rdf:Description>\n</rdf:RDF>\n");
    }
    packet.push_str("</x:xmpmeta>\n<?xpacket end=\"w\"?>");
    a_document_carrying(packet, String::new())
}

#[test]
fn a_schema_in_a_replaced_packet_takes_the_replacements_decision() {
    let source = two_rdf_elements_one_using_an_undescribed_schema();
    let target = Target::Two(Level::B);
    let replaced = Decision::Authorised {
        losses: Losses::one(Loss::MetadataPacket),
        rewrite: Rewrite::FreshMetadataPacket,
    };

    let authorised = Authorisations {
        metadata_packet: true,
        ..Authorisations::default()
    };
    let (report, output) = convert(&source, target, authorised);
    assert_eq!(decision(&report, ONE_RDF), replaced);
    assert_eq!(
        decision(&report, SCHEMAS),
        replaced,
        "the stream the schema requirement failed at is the one the fresh packet replaces"
    );
    let output = output.expect("one authorised loss, so it converts");
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    assert!(
        !String::from_utf8_lossy(&output).contains("acme"),
        "no container describes a schema the output no longer uses"
    );

    // Unauthorised, the replacement stops the conversion, and the schema row says the same
    // rather than that nothing was prepared.
    let (report, output) = convert(&source, target, Authorisations::default());
    assert!(output.is_none());
    let stopped = Decision::Unauthorised {
        losses: Losses::one(Loss::MetadataPacket),
        rewrite: Rewrite::FreshMetadataPacket,
    };
    assert_eq!(decision(&report, ONE_RDF), stopped);
    assert_eq!(decision(&report, SCHEMAS), stopped);
}

#[test]
fn the_shipped_profiles_answer_both_rows_with_the_preserved_packet() {
    // Both lower-bound profiles answer section 6.6.2.1's rows with `preserve` on a page and the
    // schema row with `preserve`, the container: the page keeps the producer's packet, the
    // `acme` property with it, and the container has nothing left to describe.
    let source = two_rdf_elements_one_using_an_undescribed_schema();
    let target = Target::Two(Level::B);
    for name in ["as-if-printed", "as-shown"] {
        let shipped = pdf_transform::profiles::named(name).expect("a shipped profile");
        let plan = plan_from(shipped.text, target);
        let (report, output) = convert_with_plan(&source, &plan);
        let Decision::Configured { rewrite, .. } = decision(&report, ONE_RDF) else {
            panic!("{name}: the profile's preserve answers the packet row");
        };
        assert_eq!(rewrite, Rewrite::FreshMetadataPacket, "{name}");
        assert_eq!(
            decision(&report, SCHEMAS),
            decision(&report, ONE_RDF),
            "{name}"
        );
        let output = output.unwrap_or_else(|| panic!("{name}: converts"));
        assert_eq!(
            holds(&output, target).verdict(),
            Verdict::Conforms,
            "{name}"
        );
    }
}

const AMENDMENT: &str = "metadata/identification-amendment-form";
const FIELDS: &str = "metadata/extension-schema-structure-fields-are-described";

/// `packet` with a second `rdf:RDF` element after its first, stating a property of a predefined
/// schema: section 6.6.2.1's fault added to whatever the first element breaks.
fn with_a_second_rdf_element(packet: &str) -> String {
    let close = "</rdf:RDF>\n</x:xmpmeta>";
    assert_eq!(packet.matches(close).count(), 1, "one place to add it");
    packet.replacen(
        close,
        "</rdf:RDF>\n<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n\
         <rdf:Description rdf:about=\"\" xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\">\n\
         <pdf:Producer>an exporter</pdf:Producer>\n</rdf:Description>\n</rdf:RDF>\n</x:xmpmeta>",
        1,
    )
}

/// A catalog packet of two `rdf:RDF` elements, the first stating a `pdfaid:amd` that is not the
/// amendment number and the year separated by a colon (section 6.6.4).
fn two_rdf_elements_one_stating_a_malformed_amendment() -> Vec<u8> {
    let mut packet = String::new();
    packet.push_str("<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n");
    packet.push_str("<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n");
    packet.push_str("<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n");
    packet.push_str("<rdf:Description rdf:about=\"\" ");
    packet.push_str("xmlns:pdfaid=\"http://www.aiim.org/pdfa/ns/id/\">\n");
    packet.push_str("<pdfaid:part>2</pdfaid:part>\n<pdfaid:conformance>B</pdfaid:conformance>\n");
    packet.push_str("<pdfaid:amd>amendment one</pdfaid:amd>\n");
    packet.push_str("</rdf:Description>\n</rdf:RDF>\n</x:xmpmeta>\n<?xpacket end=\"w\"?>");
    a_document_carrying(with_a_second_rdf_element(&packet), String::new())
}

/// The packet of `super::a_packet_whose_structure_states_an_undescribed_field`, whose value type
/// leaves out the field its structure states (section 6.6.2.3.1), given a second `rdf:RDF`.
fn two_rdf_elements_one_stating_an_undescribed_field() -> Vec<u8> {
    a_document_carrying(
        with_a_second_rdf_element(
            &super::a_packet_whose_structure_states_an_undescribed_field(
                "http://acme.example/batch/",
            ),
        ),
        String::new(),
    )
}

/// Both rows of a fixture holding `requirement`'s fault in a replaced packet take the packet row's
/// decision, authorised and not, and the authorised output conforms.
fn the_row_takes_the_replacements_decision(source: &[u8], requirement: &str) {
    let target = Target::Two(Level::B);
    let replaced = Decision::Authorised {
        losses: Losses::one(Loss::MetadataPacket),
        rewrite: Rewrite::FreshMetadataPacket,
    };
    let authorised = Authorisations {
        metadata_packet: true,
        ..Authorisations::default()
    };
    let (report, output) = convert(source, target, authorised);
    assert_eq!(decision(&report, ONE_RDF), replaced);
    assert_eq!(
        decision(&report, requirement),
        replaced,
        "the packet {requirement} failed in is the one the fresh packet replaces"
    );
    let output = output.expect("one authorised loss, so it converts");
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);

    let (report, output) = convert(source, target, Authorisations::default());
    assert!(output.is_none());
    let stopped = Decision::Unauthorised {
        losses: Losses::one(Loss::MetadataPacket),
        rewrite: Rewrite::FreshMetadataPacket,
    };
    assert_eq!(decision(&report, ONE_RDF), stopped);
    assert_eq!(decision(&report, requirement), stopped);
}

#[test]
fn an_amendment_identifier_in_a_replaced_packet_takes_the_replacements_decision() {
    let source = two_rdf_elements_one_stating_a_malformed_amendment();
    the_row_takes_the_replacements_decision(&source, AMENDMENT);
    // Not one loss more: the identifier goes with the packet, and nothing asks the caller for
    // `amendment-identifier` beside `metadata-packet`.
    let (report, _) = convert(
        &source,
        Target::Two(Level::B),
        Authorisations {
            metadata_packet: true,
            ..Authorisations::default()
        },
    );
    assert!(super::conversion(&report).removed_identifiers.is_empty());
}

#[test]
fn a_structure_field_in_a_replaced_packet_takes_the_replacements_decision() {
    let source = two_rdf_elements_one_stating_an_undescribed_field();
    the_row_takes_the_replacements_decision(&source, FIELDS);
    let (report, output) = convert(
        &source,
        Target::Two(Level::B),
        Authorisations {
            metadata_packet: true,
            ..Authorisations::default()
        },
    );
    assert!(super::conversion(&report).described_fields.is_empty());
    let output = output.expect("it converts");
    assert!(
        !String::from_utf8_lossy(&output).contains("pdfaField"),
        "no field is described in a value type the output no longer states"
    );
}

#[test]
fn the_shipped_profiles_answer_the_two_rows_with_the_preserved_packet() {
    let target = Target::Two(Level::B);
    for (source, requirement) in [
        (
            two_rdf_elements_one_stating_a_malformed_amendment(),
            AMENDMENT,
        ),
        (two_rdf_elements_one_stating_an_undescribed_field(), FIELDS),
    ] {
        for name in ["as-if-printed", "as-shown"] {
            let shipped = pdf_transform::profiles::named(name).expect("a shipped profile");
            let plan = plan_from(shipped.text, target);
            let (report, output) = convert_with_plan(&source, &plan);
            assert!(
                matches!(decision(&report, ONE_RDF), Decision::Configured { .. }),
                "{name}: the profile's preserve answers the packet row"
            );
            assert_eq!(
                decision(&report, requirement),
                decision(&report, ONE_RDF),
                "{name} {requirement}"
            );
            let output = output.unwrap_or_else(|| panic!("{name} {requirement}: converts"));
            assert_eq!(
                holds(&output, target).verdict(),
                Verdict::Conforms,
                "{name} {requirement}"
            );
        }
    }
}

#[test]
fn a_replaced_catalog_packet_that_stated_its_identification_states_it_again() {
    // The identification rows pass on the producer's packet, so no row asks for the schema; the
    // fresh packet states no property (`doc/adr/1245`), so without the schema written again the
    // output would fail section 6.6.4's part and conformance rows the source met (`doc/adr/2455`).
    let mut packet = String::new();
    packet.push_str("<?xpacket begin=\"\u{feff}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n");
    packet.push_str("<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n");
    packet.push_str("<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n");
    packet.push_str("<rdf:Description rdf:about=\"\" ");
    packet.push_str("xmlns:pdfaid=\"http://www.aiim.org/pdfa/ns/id/\">\n");
    packet.push_str("<pdfaid:part>2</pdfaid:part>\n<pdfaid:conformance>B</pdfaid:conformance>\n");
    packet.push_str("</rdf:Description>\n</rdf:RDF>\n</x:xmpmeta>\n<?xpacket end=\"w\"?>");
    let source = a_document_carrying(with_a_second_rdf_element(&packet), String::new());
    let target = Target::Two(Level::B);
    let authorised = Authorisations {
        metadata_packet: true,
        ..Authorisations::default()
    };
    let (report, output) = convert(&source, target, authorised);
    assert!(
        matches!(decision(&report, ONE_RDF), Decision::Authorised { .. }),
        "{:?}",
        decision(&report, ONE_RDF)
    );
    let output = output.expect("one authorised loss, so it converts");
    assert_eq!(holds(&output, target).verdict(), Verdict::Conforms);
    assert!(
        String::from_utf8_lossy(&output).contains("<pdfaid:part>2</pdfaid:part>"),
        "the fresh packet states the identification"
    );
}
