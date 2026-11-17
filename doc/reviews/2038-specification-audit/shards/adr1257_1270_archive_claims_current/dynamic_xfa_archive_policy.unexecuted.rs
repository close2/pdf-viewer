// Proposed focused archive integration test. This is a project-policy regression for the
// owner-ratified dynamic-XFA class, not an ISO 32000-2 mandate on every archive writer.
// Place in the existing archive.rs test module; it uses its Conforming/stream helpers.

fn xfa_pdf(template_body: &str, needs_rendering: Option<bool>) -> Vec<u8> {
    let needs = needs_rendering.map_or_else(String::new, |value| {
        format!(" /NeedsRendering {value}")
    });
    let xdp = format!(
        "<xdp:xdp xmlns:xdp=\"http://ns.adobe.com/xdp/\">\
         <template xmlns=\"http://www.xfa.org/schema/xfa-template/3.3/\">\
         <subform name=\"form1\"><subform name=\"row\">{template_body}\
         <draw name=\"label\"><value><text>Repeated row</text></value></draw>\
         </subform></subform></template>\
         <xfa:datasets xmlns:xfa=\"http://www.xfa.org/schema/xfa-data/1.0/\">\
         <xfa:data><form1><row/></form1></xfa:data></xfa:datasets></xdp:xdp>"
    );
    let data = xdp.as_bytes();
    let xfa = stream(&format!("/Length {}", data.len()), data);
    Conforming {
        catalog: format!(
            "/AcroForm << /Fields [] /XFA 6 0 R >>{needs}"
        ),
        objects: vec![String::from_utf8(xfa).expect("XFA fixture is UTF-8")],
        ..Conforming::default()
    }
    .build()
}

#[test]
fn archive_dynamic_xfa_policy_uses_template_grammar_when_the_flag_is_absent() {
    // ADR 2876 and the current owner ratification include templates outside XFAF, even when
    // Table 29's deprecated /NeedsRendering flag is absent/default false. XFA 3.3 page 333 and
    // pdf_model::xfa::Template::dynamic treats variable occurrence bounds as a dynamic marker.
    let dynamic_source = xfa_pdf("<occur min=\"1\" max=\"-1\"/>", None);
    let parsed = Document::open_with_limits(dynamic_source.clone(), Limits::DEFAULT)
        .expect("the exact-xref PDF opens");
    let catalog = parsed.catalog().expect("catalog");
    assert!(parsed.get_key(&catalog, "NeedsRendering").is_null());
    let form = parsed.get_key(&catalog, "AcroForm");
    let resource = pdf_model::xfa::Resource::read(&parsed)
        .expect("XFA resource exists")
        .expect("well-formed XDP resource");
    assert!(resource.template.dynamic().is_some(), "the fixture is grammar-dynamic");
    assert!(form.as_dict().is_some());

    // The static positive control uses the same valid wrapper but fixed occurrence bounds. With explicit XFA-loss authorization it can be removed under the existing gate.
    let static_source = xfa_pdf("<occur min=\"1\" max=\"1\"/>", None);
    let static_document = Document::open_with_limits(static_source.clone(), Limits::DEFAULT)
        .expect("the static control opens");
    let static_resource = pdf_model::xfa::Resource::read(&static_document)
        .expect("XFA resource exists")
        .expect("well-formed XDP resource");
    assert_eq!(static_resource.template.dynamic(), None);

    let mut static_authorization = Authorisations::default();
    static_authorization.authorise(Loss::XfaForm);
    let (static_report, static_output) = convert(
        &static_source,
        Target::Four(Flavour::Plain),
        static_authorization,
    );
    assert!(matches!(
        decision(&static_report, "forms/no-xfa-key"),
        Decision::Authorised { .. }
    ));
    assert!(static_output.is_some(), "the static control reaches output");

    let mut dynamic_authorization = Authorisations::default();
    dynamic_authorization.authorise(Loss::XfaForm);
    let (report, output) = convert(
        &dynamic_source,
        Target::Four(Flavour::Plain),
        dynamic_authorization,
    );
    assert!(matches!(
        decision(&report, "forms/no-xfa-key"),
        Decision::Refused(Because::Declined(_))
    ));
    assert!(
        output.is_none(),
        "the default dynamic=stop policy applies to owner-ratified grammar-dynamic XFA even without /NeedsRendering"
    );
}
