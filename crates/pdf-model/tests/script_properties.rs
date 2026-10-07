//! What a script may change on the page: the field properties ADR 1603 kept and did not draw,
//! drawn through the appearance and saved as the entries the standard draws a widget from (ADR
//! 1617).
//!
//! One fixture per property. Each document's `/OpenAction` is a script, run by a runner of this
//! test's own that answers it with one property edit — what `this.getField("Field").fillColor =
//! color.red` comes to — and the witness is the saved file: the entry ISO 32000-2 names for the
//! property, written where the standard keeps it, and the appearance constructed from it. A save
//! draws through the same construction the page does, so the saved stream is what was drawn.

#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use std::fmt::Write as _;
use std::sync::Arc;

use pdf_model::view::{
    Alignment, BorderStyle, Colour, Property, ScriptEdit, ScriptEvent, ScriptResult, ScriptRunner,
    ScriptSite, ViewState,
};
use pdf_syntax::{Dictionary, Document, Object, ObjectId};

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

/// The catalog: its open action is a script, its one field object 4, the form's `/DA` the
/// document-wide default and object 5 the font `/DR` names.
fn catalog() -> String {
    "<< /Type /Catalog /Pages 2 0 R /OpenAction << /S /JavaScript /JS (paint\\(\\);) >> \
     /AcroForm << /Fields [4 0 R] /DA (/Helv 10 Tf 0 g) /DR << /Font << /Helv 5 0 R >> >> >> >>"
        .to_owned()
}

/// One page, its annotations `annots`.
fn pages(annots: &str) -> [String; 2] {
    [
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [{annots}] >>"),
    ]
}

/// The font `/DR` names.
fn font() -> String {
    "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned()
}

/// A text field holding `Paid`, widget and field in one dictionary, with a border and a background
/// of its own and a stored appearance stream (object 6) that draws neither — the producer's
/// picture the script's change replaces.
fn text_document() -> Document {
    let [pages, page] = pages("4 0 R");
    assembled(&[
        catalog(),
        pages,
        page,
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Field) /V (Paid) /Rect [10 10 210 40] /F 4 \
         /P 3 0 R /MK << /BG [1] /BC [0] >> /AP << /N 6 0 R >> >>"
            .to_owned(),
        font(),
        "<< /Type /XObject /Subtype /Form /BBox [0 0 200 30] /Length 46 >>\nstream\n\
         /Tx BMC BT /Helv 10 Tf 2 10 Td (Paid) Tj ET EMC\nendstream"
            .to_owned(),
    ])
}

/// A text field (object 4) whose one widget is a kid of it (object 6).
fn kid_document() -> Document {
    let [pages, page] = pages("6 0 R");
    assembled(&[
        catalog(),
        pages,
        page,
        "<< /FT /Tx /T (Field) /V (Paid) /Kids [6 0 R] >>".to_owned(),
        font(),
        "<< /Type /Annot /Subtype /Widget /Parent 4 0 R /Rect [10 10 210 40] /F 4 /P 3 0 R \
         /MK << /BG [1] /BC [0] >> >>"
            .to_owned(),
    ])
}

/// A runner answering the open action with properties set on `Field`.
#[derive(Debug)]
struct Setting(Vec<Property>);

impl ScriptRunner for Setting {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        let edits = if event.site == ScriptSite::OpenAction {
            self.0
                .iter()
                .map(|property| ScriptEdit::Property {
                    field: "Field".to_owned(),
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

/// The saved file after the open action set `properties`.
fn saved_all(document: &Document, properties: Vec<Property>) -> Document {
    let mut view = ViewState::of(document);
    view.run_scripts_with(Some(Arc::new(Setting(properties))));
    view.run_open_scripts(document, 0);
    let written = view.save(document).expect("the update writes");
    Document::open(written.bytes).expect("the update reads back")
}

/// Object `number` of `saved`, a dictionary.
fn object(saved: &Document, number: u32) -> Dictionary {
    saved
        .get(ObjectId {
            number,
            generation: 0,
        })
        .as_dict()
        .cloned()
        .expect("a dictionary")
}

/// The saved file after the open action set `property`, and its widget, object 4.
fn saved(document: &Document, property: Property) -> (Document, Dictionary) {
    let saved = saved_all(document, vec![property]);
    let widget = object(&saved, 4);
    (saved, widget)
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

/// The numbers of an array entry, or `None` where it is not one.
fn numbers(saved: &Document, dict: &Dictionary, key: &str) -> Option<Vec<f64>> {
    let entry = saved.get_key(dict, key);
    entry
        .as_array()?
        .iter()
        .map(|item| match item {
            Object::Integer(value) => Some(f64::from(i32::try_from(*value).ok()?)),
            Object::Real(value) => Some(*value),
            _ => None,
        })
        .collect()
}

/// The widget's `/MK`, as the saved file holds it.
fn characteristics(saved: &Document, widget: &Dictionary) -> Dictionary {
    saved
        .get_key(widget, "MK")
        .as_dict()
        .cloned()
        .expect("an /MK")
}

/// `fillColor`: Table 192's `/BG`, and the stored stream set aside for one drawing the colour.
#[test]
fn a_fill_colour_is_the_background_and_is_drawn() {
    let document = text_document();
    let (saved, widget) = saved(&document, Property::FillColor(Colour::Rgb([1.0, 0.0, 0.0])));
    assert_eq!(
        numbers(&saved, &characteristics(&saved, &widget), "BG"),
        Some(vec![1.0, 0.0, 0.0])
    );
    let drawn = appearance(&saved, &widget);
    assert!(
        drawn.contains("1 0 0 rg"),
        "the background is filled red: {drawn}"
    );
    assert!(
        drawn.contains("(Paid) Tj"),
        "and the value is still drawn: {drawn}"
    );
}

/// `strokeColor`: Table 192's `/BC`, stroked as the border.
#[test]
fn a_stroke_colour_is_the_border_and_is_drawn() {
    let document = text_document();
    let (saved, widget) = saved(
        &document,
        Property::StrokeColor(Colour::Rgb([0.0, 0.0, 1.0])),
    );
    assert_eq!(
        numbers(&saved, &characteristics(&saved, &widget), "BC"),
        Some(vec![0.0, 0.0, 1.0])
    );
    let drawn = appearance(&saved, &widget);
    assert!(
        drawn.contains("0 0 1 RG"),
        "the border is stroked blue: {drawn}"
    );
}

/// `textColor`: a colour operator after the `/DA`'s own, written on the field, and the text drawn
/// in it.
#[test]
fn a_text_colour_is_a_colour_after_the_default_appearance() {
    let document = text_document();
    let (saved, widget) = saved(&document, Property::TextColor(Colour::Rgb([0.0, 0.5, 0.0])));
    let stated = saved.get_key(&widget, "DA");
    let default_appearance = stated.as_string().expect("a /DA on the field");
    assert_eq!(default_appearance, b"/Helv 10 Tf 0 g 0 0.5 0 rg");
    let drawn = appearance(&saved, &widget);
    assert!(
        drawn.contains("0 0.5 0 rg"),
        "the text is drawn green: {drawn}"
    );
}

/// `borderStyle`: Table 168's `/S` in the widget's `/BS`.
#[test]
fn a_border_style_is_table_168s_name() {
    let document = text_document();
    let (saved, widget) = saved(&document, Property::BorderStyle(BorderStyle::Dashed));
    let style = saved.get_key(&widget, "BS");
    let style = style.as_dict().expect("a /BS");
    assert_eq!(
        saved
            .get_key(style, "S")
            .as_name()
            .map(pdf_syntax::Name::as_bytes),
        Some(&b"D"[..])
    );
    let drawn = appearance(&saved, &widget);
    assert!(drawn.contains("] 0 d"), "the border is dashed: {drawn}");
}

/// `alignment`: Table 228's `/Q`, and the text set right of where a left quadding puts it.
#[test]
fn an_alignment_is_the_quadding() {
    let document = text_document();
    let (saved, widget) = saved(&document, Property::Alignment(Alignment::Right));
    assert_eq!(saved.get_key(&widget, "Q").as_integer(), Some(2));
    let (left, _) = self::saved(&document, Property::Alignment(Alignment::Left));
    let start = |drawn: &str| {
        drawn
            .split_whitespace()
            .collect::<Vec<_>>()
            .windows(7)
            .find(|words| words[6] == "Tm")
            .and_then(|words| words[4].parse::<f64>().ok())
            .unwrap_or_else(|| panic!("a Tm in {drawn}"))
    };
    let right_start = start(&appearance(&saved, &widget));
    let left_widget = left
        .get(ObjectId {
            number: 4,
            generation: 0,
        })
        .as_dict()
        .cloned()
        .expect("the widget");
    let left_start = start(&appearance(&left, &left_widget));
    assert!(
        right_start > left_start,
        "right-quadded text starts further right: {right_start} against {left_start}"
    );
}

/// `charLimit`: Table 232's `/MaxLen`.
#[test]
fn a_character_limit_is_the_maximum_length() {
    let document = text_document();
    let (saved, widget) = saved(&document, Property::CharLimit(5));
    assert_eq!(saved.get_key(&widget, "MaxLen").as_integer(), Some(5));
}

/// `required`: Table 227's bit 2 in the field's `/Ff`, the other bits as they were.
#[test]
fn required_is_table_227s_bit_two() {
    let document = text_document();
    let (saved, widget) = saved(&document, Property::Required(true));
    assert_eq!(saved.get_key(&widget, "Ff").as_integer(), Some(2));
}

/// A field whose widget is a kid: the field's entries go on the field, which states `/T`, and the
/// widget's on the widget — so another widget of the field would inherit the first and not the
/// second, as §12.7.4.1's inheritance reads them.
#[test]
fn a_fields_entries_go_on_the_field_and_a_widgets_on_the_widget() {
    let saved = saved_all(
        &kid_document(),
        vec![
            Property::Alignment(Alignment::Center),
            Property::FillColor(Colour::Gray(0.5)),
        ],
    );
    let field = object(&saved, 4);
    let widget = object(&saved, 6);
    assert_eq!(saved.get_key(&field, "Q").as_integer(), Some(1));
    assert!(
        widget.get("Q").is_none(),
        "the field's entry is not on the widget"
    );
    assert_eq!(
        numbers(&saved, &characteristics(&saved, &widget), "BG"),
        Some(vec![0.5])
    );
    assert!(
        field.get("MK").is_none(),
        "the widget's entry is not on the field"
    );
    let drawn = appearance(&saved, &widget);
    assert!(
        drawn.contains("0.5 g"),
        "the grey background is drawn: {drawn}"
    );
}
