//! Specification audit of ISO 32000-2 §7.3's object syntax in what the form writer emits into a
//! `/DA` string and a field's appearance stream (review 2243,
//! `doc/reviews/2243-specification-audit-7-1-7-3.md`).
//!
//! A `/DA` is a content-stream fragment (§12.7.4.3), so the names and numbers written into it are
//! read back by §7.2's lexer and §7.3's object rules like any other. Each `*_spec_audit` test fails
//! on the tree it was written against and has a passing control beside it.

#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use std::fmt::Write as _;
use std::sync::Arc;

use pdf_model::view::{
    Property, ScriptEdit, ScriptEvent, ScriptResult, ScriptRunner, ScriptSite, ViewState,
};
use pdf_syntax::{Dictionary, Document, ObjectId};

/// A document of `objects`, numbered from 1, the first the catalog.
fn assembled(objects: &[String]) -> Document {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = objects.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    Document::open(out.into_bytes()).expect("the fixture opens")
}

/// A text field `Field` (object 4, widget and field in one) whose form states `default_appearance`
/// as its `/DA` and whose `/DR` holds object 5 under the key `font_key` (written as §7.3.5 escapes
/// it). The catalog's open action is a script, so a runner can set properties on open.
fn text_document(default_appearance: &str, font_key: &str) -> Document {
    assembled(&[
        format!(
            "<< /Type /Catalog /Pages 2 0 R /OpenAction << /S /JavaScript /JS (paint\\(\\);) >> \
             /AcroForm << /Fields [4 0 R] /DA ({default_appearance}) \
             /DR << /Font << /{font_key} 5 0 R >> >> >> >>"
        ),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [4 0 R] >>".to_owned(),
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Field) /V (Paid) /Rect [10 10 210 40] /F 4 \
         /P 3 0 R >>"
            .to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
    ])
}

/// A runner answering the open action with `properties` set on `Field`.
#[derive(Debug)]
struct Setting(Vec<Property>);

impl ScriptRunner for Setting {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        let edits = if event.site == ScriptSite::OpenAction {
            self.0
                .iter()
                .map(|property| ScriptEdit::Property {
                    field: "Field".to_owned(),
                    widget: None,
                    property: property.clone(),
                })
                .collect()
        } else {
            Vec::new()
        };
        ScriptResult {
            rc: true,
            value: None,
            change: None,
            edits,
            report: Vec::new(),
        }
    }
}

/// The saved file after the open action set `properties`, and its widget (object 4).
fn saved(document: &Document, properties: Vec<Property>) -> (Document, Dictionary) {
    let mut view = ViewState::of(document);
    view.run_scripts_with(Some(Arc::new(Setting(properties))));
    view.run_open_scripts(document, 0);
    let written = view.save(document).expect("the update writes");
    let saved = Document::open(written.bytes).expect("the update reads back");
    let widget = saved
        .get(ObjectId::new(4, 0))
        .as_dict()
        .cloned()
        .expect("the widget");
    (saved, widget)
}

/// The `/DA` the save wrote on the field.
fn default_appearance(saved: &Document, widget: &Dictionary) -> String {
    let stated = saved.get_key(widget, "DA");
    String::from_utf8_lossy(stated.as_string().expect("a /DA on the field")).into_owned()
}

/// The decoded normal appearance the save wrote for the widget.
fn appearance(saved: &Document, widget: &Dictionary) -> String {
    let appearances = saved.get_key(widget, "AP");
    let normal = appearances
        .as_dict()
        .map(|appearances| saved.get_key(appearances, "N"))
        .expect("an /AP");
    let stream = normal.as_stream().expect("a stream");
    String::from_utf8_lossy(&saved.decoded_stream_data(stream).expect("decodes")).into_owned()
}

/// ISO 32000-2 §7.3.5: "Any character that is not a regular character shall be written using its
/// 2-digit hexadecimal code, preceded by the NUMBER SIGN only", and "[w]hitespace used as part of a
/// name shall always be coded using the 2-digit hexadecimal notation". A script that sets only
/// `textSize` has the producer's own `/DA` font re-stated in a new `Tf` — and that name, the `/DR`
/// key `Helv Bold`, is written with its SPACE raw: `/Helv Bold 14 Tf` is the name `Helv`, the
/// operator `Bold` and a `Tf` whose first operand is a number.
#[test]
fn a_restated_default_appearance_font_name_is_escaped_spec_audit() {
    let document = text_document("/Helv#20Bold 10 Tf 0 g", "Helv#20Bold");
    let (saved, widget) = saved(&document, vec![Property::TextSize(14.0)]);
    let stated = default_appearance(&saved, &widget);
    assert!(
        stated.ends_with("/Helv#20Bold 14 Tf"),
        "the restated Tf names the producer's font with §7.3.5's escape: {stated}"
    );
}

/// The control: a font name needing no escape is restated as itself.
#[test]
fn a_restated_default_appearance_font_name_with_regular_characters_stands() {
    let document = text_document("/Helv 10 Tf 0 g", "Helv");
    let (saved, widget) = saved(&document, vec![Property::TextSize(14.0)]);
    let stated = default_appearance(&saved, &widget);
    assert!(stated.ends_with("/Helv 14 Tf"), "{stated}");
}

/// ISO 32000-2 §7.3.3: "A real value shall be written as one or more decimal digits with an
/// optional sign and a leading, trailing, or embedded PERIOD (2Eh) (decimal point)." A `/DA` real
/// the writer replays into the appearance stream is narrowed to `f32` first, so one past `f32`'s
/// range — legal syntax, Annex C's limits being advice — is written as the three letters `inf`:
/// not a number at all, but an operator token in the stream it lands in.
#[test]
fn a_replayed_default_appearance_real_is_written_as_a_number_spec_audit() {
    let document = text_document(
        "/Helv 10 Tf 400000000000000000000000000000000000000.5 w 0 g",
        "Helv",
    );
    let (saved, widget) = saved(&document, vec![Property::TextSize(12.0)]);
    let drawn = appearance(&saved, &widget);
    assert!(
        !drawn.contains("inf"),
        "every operand the appearance replays is a §7.3.3 number: {drawn}"
    );
    assert!(drawn.contains(" w"), "the line width is replayed: {drawn}");
}

/// The control: a real inside `f32`'s range is replayed as a number.
#[test]
fn a_replayed_default_appearance_real_in_range_is_a_number() {
    let document = text_document("/Helv 10 Tf 2.5 w 0 g", "Helv");
    let (saved, widget) = saved(&document, vec![Property::TextSize(12.0)]);
    let drawn = appearance(&saved, &widget);
    assert!(drawn.contains("2.5 w"), "{drawn}");
}
