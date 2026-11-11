//! ISO 32000-2 Annex K.2: whole-stream and packet-array XFA resources.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test fixtures must fail loudly when the PDF or XFA resource cannot be read"
)]

use std::fmt::Write as _;

use pdf_model::xfa::Resource;
use pdf_syntax::{Document, Object};

const FIELD: &str = "form1[0].name[0]";
const TEMPLATE: &str = "<template xmlns=\"http://www.xfa.org/schema/xfa-template/3.3/\"><subform name=\"form1\"><field name=\"name\"><ui><textEdit/></ui><value><text>Template default</text></value></field></subform></template>";
const DATASETS: &str = "<xfa:datasets xmlns:xfa=\"http://www.xfa.org/schema/xfa-data/1.0/\"><xfa:data><form1><name>Ada</name></form1></xfa:data></xfa:datasets>";
const OPEN: &str = "<xdp:xdp xmlns:xdp=\"http://ns.adobe.com/xdp/\">";
const CLOSE: &str = "</xdp:xdp>";

/// Encodes XML as a PDF stream with its exact uncompressed Length.
fn pdf_stream(xml: &str) -> String {
    format!("<< /Length {} >>\nstream\n{}\nendstream", xml.len(), xml)
}

/// Builds a complete small PDF from object bodies and a correctly offset classic xref table.
fn pdf(objects: &[String]) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        write!(out, "{} 0 obj\n{body}\nendobj\n", index + 1).expect("String writes succeed");
    }
    let xref_at = out.len();
    write!(out, "xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1)
        .expect("String writes succeed");
    for offset in offsets {
        writeln!(out, "{offset:010} 00000 n ").expect("String writes succeed");
    }
    write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
        objects.len() + 1
    )
    .expect("String writes succeed");
    out.into_bytes()
}

/// A PDF field value inherited through the test field's parent, as text.
fn pdf_field_value(document: &Document) -> Option<String> {
    let widgets = pdf_model::view::widgets_by_field_name(document);
    let widget = *widgets.get(FIELD)?.first()?;
    let object = document.get(widget);
    let dictionary = object.as_dict()?;
    match document.get_key(dictionary, "V") {
        Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
        _ => None,
    }
}

/// A full one-page hybrid PDF whose /XFA entry refers to the supplied objects.
fn document(xfa_entry: &str, resource_objects: &[String]) -> Document {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /AcroForm 4 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> /Contents 5 0 R /Annots [7 0 R] >>".to_owned(),
        format!("<< /Fields [6 0 R] /XFA {xfa_entry} /DA (/Helv 10 Tf 0 g) /DR << /Font << /Helv << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> >>"),
        "<< /Length 0 >>\nstream\n\nendstream".to_owned(),
        "<< /T (form1[0]) /FT /Tx /Kids [7 0 R] >>".to_owned(),
        "<< /Type /Annot /Subtype /Widget /T (name[0]) /Parent 6 0 R /P 3 0 R /Rect [72 700 200 720] /V (Ada) /DA (/Helv 10 Tf 0 g) /AP << /N 8 0 R >> >>".to_owned(),
        "<< /Type /XObject /Subtype /Form /BBox [0 0 128 20] /Resources << /Font << /Helv << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> /Length 33 >>\nstream\nBT /Helv 10 Tf 0 0 Td (Ada) Tj ET\nendstream".to_owned(),
    ];
    objects.extend_from_slice(resource_objects);
    let parsed = Document::open(pdf(&objects)).expect("the complete fixture parses");
    assert!(!parsed.was_recovered(), "the fixture's xref is valid");
    let page = pdf_model::Pages::new(&parsed)
        .get(0)
        .expect("the fixture has one page");
    assert!(
        pdf_model::interpret(&parsed, &page).is_complete(),
        "the page interpretation is complete"
    );
    assert_eq!(pdf_field_value(&parsed).as_deref(), Some("Ada"));
    parsed
}

/// Whole-XDP and named-packet inputs both parse the same named text field, dataset value and
/// corresponding PDF `/V`. This fails if either form is recognized only by key presence or
/// silently defaults a missing template/datasets packet.
fn resources() -> [Document; 2] {
    let whole = format!("{OPEN}{TEMPLATE}{DATASETS}{CLOSE}");
    let stream = document("9 0 R", &[pdf_stream(&whole)]);
    let packets = document(
        "[(xdp:xdp) 9 0 R (template) 10 0 R (datasets) 11 0 R (/xdp:xdp) 12 0 R]",
        &[
            pdf_stream(OPEN),
            pdf_stream(TEMPLATE),
            pdf_stream(DATASETS),
            pdf_stream(CLOSE),
        ],
    );

    [stream, packets]
}

#[test]
fn whole_xdp_stream_and_named_packets_read_the_same_named_field_and_value() {
    for (label, document) in ["whole stream", "packet array"]
        .into_iter()
        .zip(resources().iter())
    {
        let resource = Resource::read(document)
            .expect("the catalog states /AcroForm /XFA")
            .expect("the XDP resource has a template and datasets packet");
        assert_eq!(resource.template.dynamic(), None, "{label} is static");
        assert_eq!(
            resource
                .datasets
                .value(&resource.template, FIELD)
                .as_deref(),
            Some("Ada"),
            "{label} binds the named XFA text field to its nonempty data value"
        );
        assert_eq!(
            pdf_field_value(document).as_deref(),
            Some("Ada"),
            "{label} /V"
        );
        assert!(
            document.get(resource.datasets_stream).as_stream().is_some(),
            "{label} resolves its datasets source stream"
        );
    }
}

#[test]
fn saving_both_resource_forms_preserves_template_and_updates_data_and_pdf_v() {
    use pdf_model::view::{Entered, ViewState};
    for (label, document) in ["whole stream", "packet array"]
        .into_iter()
        .zip(resources())
    {
        let changed = "Grace & <Hopper>";
        let mut view = ViewState::of(&document);
        assert!(
            view.set_field(&document, FIELD, &Entered::Text(changed.to_owned())) > 0,
            "{label}: field accepts change"
        );
        let saved = view.save(&document).expect("hybrid form saves");
        assert!(
            saved.xfa_unkept.is_empty(),
            "{label}: {:?}",
            saved.xfa_unkept
        );
        assert!(
            saved
                .bytes
                .starts_with(document.bytes().whole().expect("source bytes"))
        );
        let saved = Document::open(saved.bytes).expect("saved file opens");
        assert!(!saved.was_recovered(), "{label}: valid appended xref");
        assert_eq!(
            pdf_field_value(&saved).as_deref(),
            Some(changed),
            "{label}: saved PDF /V"
        );
        let resource = Resource::read(&saved)
            .expect("XFA remains present")
            .expect("template and data remain readable");
        assert_eq!(
            resource
                .datasets
                .value(&resource.template, FIELD)
                .as_deref(),
            Some(changed),
            "{label}: saved XFA value"
        );
        assert_eq!(
            resource.template.dynamic(),
            None,
            "{label}: static template retained"
        );
    }
}
