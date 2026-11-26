//! Annex K's rules on a writer that modifies a file holding an XFA resource (audit 2271).
//!
//! ISO 32000-2 Annex K: "The other entries in the interactive form dictionary shall be consistent
//! with the information in the XFA resource. When creating or modifying a PDF file with an XFA
//! resource, a PDF writer shall follow these rules", the first of which is "PDF interactive form
//! field objects shall be present for each field specified in the XFA resource. The XFA field
//! values shall be consistent with the corresponding V entries of the PDF field objects."

#![expect(
    clippy::expect_used,
    reason = "test code: a fixture that cannot be built must fail loudly"
)]

use std::fmt::Write as _;

use pdf_syntax::{Document, Limits, Object, ObjectId};
use pdf_transform::redact::RedactPlan;
use pdf_transform::{Budget, MemorySinks, Plan, Policy, Source, apply};

/// The value the field and its XFA data both hold.
const SECRET: &str = "SECRETX";

fn assemble(objects: &[String]) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{object}\nendobj", index.saturating_add(1));
    }
    let at = out.len();
    let size = objects.len().saturating_add(1);
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{at}\n%%EOF\n"
    );
    out.into_bytes()
}

fn stream(entries: &str, data: &str) -> String {
    format!(
        "<< {entries} /Length {} >>\nstream\n{data}\nendstream",
        data.len()
    )
}

/// A hybrid form: one text field `name`, merged with its widget, whose `/V` is [`SECRET`], and an
/// XFA resource whose `datasets` packet holds the same value for the same field; a `/Redact`
/// annotation covers the widget.
fn hybrid_form_with_a_redacted_field() -> Vec<u8> {
    let xfa = format!(
        "<xdp:xdp xmlns:xdp=\"http://ns.adobe.com/xdp/\">\
         <template xmlns=\"http://www.xfa.org/schema/xfa-template/3.3/\">\
         <subform name=\"form1\"><field name=\"name\"><ui><textEdit/></ui></field></subform>\
         </template>\
         <xfa:datasets xmlns:xfa=\"http://www.xfa.org/schema/xfa-data/1.0/\"><xfa:data>\
         <form1><name>{SECRET}</name></form1></xfa:data></xfa:datasets></xdp:xdp>"
    );
    let objects = vec![
        // 1
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] /XFA 7 0 R >> >>".to_owned(),
        // 2
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        // 3
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> /Contents 4 0 R \
         /Annots [5 0 R 6 0 R] >>"
            .to_owned(),
        // 4
        stream("", "0 0 1 1 re f"),
        // 5: the field and its widget, inside the region.
        format!(
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (form1[0].name[0]) /V ({SECRET}) \
             /Rect [100 95 140 115] /P 3 0 R >>"
        ),
        // 6
        "<< /Type /Annot /Subtype /Redact /Rect [90 90 150 120] >>".to_owned(),
        // 7
        stream("", &xfa),
    ];
    assemble(&objects)
}

/// Every decoded stream of the output that still holds [`SECRET`], by object number.
fn streams_holding_the_value(bytes: &[u8]) -> Vec<u32> {
    let document = Document::open_with_limits(bytes.to_vec(), Limits::DEFAULT).expect("it opens");
    let size = document
        .trailer()
        .get("Size")
        .and_then(Object::as_integer)
        .and_then(|size| u32::try_from(size).ok())
        .expect("a /Size");
    (1..size)
        .filter(|&number| {
            let object = document.get(ObjectId {
                number,
                generation: 0,
            });
            object.as_stream().is_some_and(|stream| {
                let data = document
                    .decoded_stream_data(stream)
                    .expect("every stream decodes");
                data.windows(SECRET.len())
                    .any(|window| window == SECRET.as_bytes())
            })
        })
        .collect()
}

/// 2271-B04. The redaction removes the widget and its field's `/V` (ADR 3725), and carries the
/// interactive form dictionary's `/XFA` with the field's value still in its `datasets` packet.
/// The written file then specifies an XFA field with no PDF field object and an XFA value no `/V`
/// agrees with, which Annex K's first rule forbids a writer that modifies the file; and the value
/// the region held is still in the file, where §12.5.6.23 says the processor "shall remove all
/// traces of the specified content".
#[test]
fn a_redacted_fields_xfa_value_does_not_survive_spec_audit() {
    let source = hybrid_form_with_a_redacted_field();
    // The control: the source holds the value in the XFA stream (object 7).
    assert_eq!(streams_holding_the_value(&source), vec![7]);

    let sinks = MemorySinks::new();
    apply(
        &Plan::Redact(RedactPlan {
            source: 0,
            names: "out.pdf".parse().expect("a pattern"),
            image_samples: None,
        }),
        &[Source::new(source)],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("the redaction applies");
    let mut outputs = sinks.into_outputs();
    assert_eq!(outputs.len(), 1, "one source, one output");
    let written = outputs.remove(0).1;
    let raw = written
        .windows(SECRET.len())
        .any(|window| window == SECRET.as_bytes());
    let held = streams_holding_the_value(&written);
    // Which object the output's interactive form dictionary names as its XFA resource, so that a
    // failure says the value is in that packet rather than in something the region left.
    let document = Document::open_with_limits(written.clone(), Limits::DEFAULT).expect("it opens");
    let catalog = document.catalog().expect("a catalog");
    let form = document.get_key(&catalog, "AcroForm");
    let xfa = form
        .as_dict()
        .and_then(|form| form.get("XFA"))
        .and_then(Object::as_reference)
        .map(|id| id.number);
    let fields = form
        .as_dict()
        .map(|form| document.get_key(form, "Fields"))
        .and_then(|fields| fields.as_array().map(<[Object]>::len));
    assert!(
        !raw && held.is_empty(),
        "Annex K: the XFA value of a field whose PDF field object the redaction removed is still \
         written (raw bytes: {raw}, decoded streams {held:?}, the /XFA stream {xfa:?}, \
         /Fields holding {fields:?} field(s))"
    );
}
