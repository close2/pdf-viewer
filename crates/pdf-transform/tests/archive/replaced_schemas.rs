//! A packet section 6.6.2.1 replaces meets section 6.6.2.3.2 by the replacement.
//!
//! ISO 19005-2 section 6.6.2.3.2 requires every extension schema a metadata stream uses to be
//! described in that stream or in the catalog's. Section 6.6.2.1 requires the packet to be one
//! `rdf:RDF` element, and a packet that is not is replaced by one stating no property at all
//! (`doc/adr/1245`) — which uses no extension schema, so the replacement is what meets the
//! schema requirement at that stream, and the decision is the replacement's own rather than a
//! refusal that no container was prepared (`doc/adr/2442`).
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
