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
    Alignment, BorderStyle, Colour, FieldState, FocusRequest, Glyph, Property, ScriptEdit,
    ScriptEvent, ScriptResult, ScriptRunner, ScriptSite, ViewState,
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

/// A text field (object 4) with two widgets that are kids of it (objects 6 and 7), the second
/// further down the page.
fn two_widget_document() -> Document {
    let [pages, page] = pages("6 0 R 7 0 R");
    assembled(&[
        catalog(),
        pages,
        page,
        "<< /FT /Tx /T (Field) /V (Paid) /Kids [6 0 R 7 0 R] >>".to_owned(),
        font(),
        "<< /Type /Annot /Subtype /Widget /Parent 4 0 R /Rect [10 10 210 40] /F 4 /P 3 0 R \
         /MK << /BG [1] /BC [0] >> >>"
            .to_owned(),
        "<< /Type /Annot /Subtype /Widget /Parent 4 0 R /Rect [10 60 210 90] /F 4 /P 3 0 R \
         /MK << /BG [1] /BC [0] >> >>"
            .to_owned(),
    ])
}

/// A runner answering the open action with properties set through a `Field` of one widget —
/// `this.getField("Field.1").fillColor = color.red` — and keeping what it was told of `Field`.
#[derive(Debug, Default)]
struct OneWidget {
    /// The widget, and what was set on it.
    set: (u32, Vec<Property>),
    /// The field's state as the open action was told it.
    told: std::sync::Mutex<Option<FieldState>>,
}

impl ScriptRunner for OneWidget {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        if let Some(field) = event.fields.iter().find(|field| field.name == "Field")
            && let Ok(mut told) = self.told.lock()
        {
            *told = Some(field.clone());
        }
        let edits = if event.site == ScriptSite::OpenAction {
            self.set
                .1
                .iter()
                .map(|property| ScriptEdit::Property {
                    field: "Field".to_owned(),
                    widget: Some(self.set.0),
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

/// A realm is told each widget of a field, and a property a script set through one widget's
/// `Field` is drawn and saved on that widget alone: its `/MK` is the widget's, and the `/DA` its
/// text colour would be written to is the field's, which the other widget would inherit, so it is
/// not written there and the widget's own appearance carries the colour (ADR 1664).
#[test]
fn a_property_set_on_one_widget_reaches_that_widget_alone() {
    let document = two_widget_document();
    let runner = Arc::new(OneWidget {
        set: (
            1,
            vec![
                Property::FillColor(Colour::Rgb([1.0, 0.0, 0.0])),
                Property::TextColor(Colour::Rgb([0.0, 0.0, 1.0])),
            ],
        ),
        ..OneWidget::default()
    });
    let mut view = ViewState::of(&document);
    view.run_scripts_with(Some(Arc::clone(&runner) as Arc<dyn ScriptRunner>));
    view.run_open_scripts(&document, 0);
    let told = runner
        .told
        .lock()
        .ok()
        .and_then(|told| told.clone())
        .expect("the realm is told of the field");
    let rects: Vec<[f64; 4]> = told.widgets.iter().map(|widget| widget.rect).collect();
    assert_eq!(
        rects,
        vec![[10.0, 10.0, 210.0, 40.0], [10.0, 60.0, 210.0, 90.0]]
    );

    let written = view.save(&document).expect("the update writes");
    let saved = Document::open(written.bytes).expect("the update reads back");
    let (first, second, field) = (object(&saved, 6), object(&saved, 7), object(&saved, 4));
    assert_eq!(
        numbers(&saved, &characteristics(&saved, &second), "BG"),
        Some(vec![1.0, 0.0, 0.0])
    );
    assert_eq!(
        numbers(&saved, &characteristics(&saved, &first), "BG"),
        Some(vec![1.0]),
        "the first widget keeps its own background"
    );
    assert!(
        field.get("DA").is_none(),
        "no text colour is written where the first widget would inherit it"
    );
    let drawn = appearance(&saved, &second);
    assert!(drawn.contains("1 0 0 rg"), "the red background: {drawn}");
    assert!(drawn.contains("0 0 1 rg"), "the blue text: {drawn}");
}

/// A runner answering the open action with `setFocus` through a `Field` of the widget it holds —
/// `this.getField("Field.1").setFocus()` for `Some(1)`.
#[derive(Debug)]
struct Focusing(Option<u32>);

impl ScriptRunner for Focusing {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        let edits = if event.site == ScriptSite::OpenAction {
            vec![ScriptEdit::Focus {
                field: "Field".to_owned(),
                widget: self.0,
            }]
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

/// A focus request through one widget's `Field` is handed on with that widget's place in the
/// field table's order, a `Field` of every widget asks for the first, and a place past the
/// field's last widget is reported and asks for nothing (ADR 1688).
#[test]
fn a_focus_request_names_the_widget_its_field_stood_for() {
    let document = two_widget_document();
    for (asked, handed) in [(Some(1), Some(1)), (None, Some(0)), (Some(2), None)] {
        let mut view = ViewState::of(&document);
        view.run_scripts_with(Some(Arc::new(Focusing(asked))));
        view.run_open_scripts(&document, 0);
        assert_eq!(
            view.take_focus_request(),
            handed.map(|widget| FocusRequest {
                field: "Field".to_owned(),
                widget,
            }),
            "{asked:?}"
        );
        assert_eq!(view.take_focus_request(), None, "a request is taken once");
        assert_eq!(
            view.script_reports()
                .iter()
                .any(|sentence| sentence.contains("its widget 2, and the field has 2")),
            handed.is_none(),
            "{:?}",
            view.script_reports()
        );
    }
}

/// A check box (object 4), on, whose `/DA` selects `face` and whose stored states draw a tick
/// (object 6) and nothing (object 7); its `/MK` names the tick's code as its caption.
fn check_box_document(face: &str) -> Document {
    check_box_in(face, "Yes")
}

/// [`check_box_document`], its value and `/AS` naming `state`.
fn check_box_in(face: &str, state: &str) -> Document {
    let [pages, page] = pages("4 0 R");
    assembled(&[
        catalog(),
        pages,
        page,
        format!(
            "<< /Type /Annot /Subtype /Widget /FT /Btn /T (Field) /V /{state} /AS /{state} \
             /Rect [10 10 30 30] /F 4 /P 3 0 R /DA ({face} 0 Tf 0 g) \
             /MK << /BG [1] /BC [0] /CA (4) >> \
             /AP << /N << /Yes 6 0 R /Off 7 0 R >> /D << /Yes 6 0 R /Off 7 0 R >> >> >>"
        ),
        font(),
        "<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] /Length 46 >>\nstream\n\
         BT /ZaDb 14 Tf 3 4 Td (4) Tj ET 0 0 20 20 re S\nendstream"
            .to_owned(),
        "<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] /Length 14 >>\nstream\n\
         0 0 20 20 re S\nendstream"
            .to_owned(),
    ])
}

/// A runner answering the open action with `this.getField("Field").checkThisBox(0)` as the
/// realm makes it — the value the widget's on state is selected by — and keeping what it was told.
#[derive(Debug, Default)]
struct Checking {
    /// The field's state as the open action was told it.
    told: std::sync::Mutex<Option<FieldState>>,
}

impl ScriptRunner for Checking {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        if let Some(field) = event.fields.iter().find(|field| field.name == "Field")
            && let Ok(mut told) = self.told.lock()
        {
            *told = Some(field.clone());
        }
        let edits = if event.site == ScriptSite::OpenAction {
            vec![ScriptEdit::Value {
                field: "Field".to_owned(),
                value: "Yes".to_owned(),
            }]
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

/// A realm is told each toggling widget's §12.7.5.2.3 on state, and the value `checkThisBox` sets
/// from it is saved as that state: `/V` and `/AS` both name it (ADR 1689).
#[test]
fn a_check_box_s_on_state_is_told_and_checking_it_saves_that_state() {
    let document = check_box_in("/ZaDb", "Off");
    let runner = Arc::new(Checking::default());
    let mut view = ViewState::of(&document);
    view.run_scripts_with(Some(Arc::clone(&runner) as Arc<dyn ScriptRunner>));
    view.run_open_scripts(&document, 0);
    let told = runner
        .told
        .lock()
        .ok()
        .and_then(|told| told.clone())
        .expect("the realm is told of the field");
    assert_eq!(told.value, "Off");
    assert_eq!(
        told.widgets
            .iter()
            .map(|widget| widget.on_state.clone())
            .collect::<Vec<_>>(),
        vec![Some("Yes".to_owned())]
    );
    let written = view.save(&document).expect("the update writes");
    let saved = Document::open(written.bytes).expect("the update reads back");
    let widget = object(&saved, 4);
    for key in ["V", "AS"] {
        assert_eq!(
            saved
                .get_key(&widget, key)
                .as_name()
                .map(|name| name.as_bytes().to_vec()),
            Some(b"Yes".to_vec()),
            "{key}: {widget:?}"
        );
    }
}

/// The decoded stream a saved widget's `/AP /N` names for appearance state `state`.
fn state_appearance(saved: &Document, widget: &Dictionary, state: &str) -> String {
    let states = normal_states(saved, widget);
    let stream = saved.get_key(&states, state);
    let stream = stream.as_stream().expect("a stream");
    String::from_utf8_lossy(&saved.decoded_stream_data(stream).expect("decodes")).into_owned()
}

/// A saved widget's `/AP /N`, which §12.7.5.2.3 makes a dictionary of states for a check box.
fn normal_states(saved: &Document, widget: &Dictionary) -> Dictionary {
    let appearances = saved.get_key(widget, "AP");
    let normal = appearances
        .as_dict()
        .map(|appearances| saved.get_key(appearances, "N"));
    // The variant itself, because `Object::as_dict` answers for a stream too.
    normal
        .and_then(|normal| match normal {
            Object::Dictionary(states) => Some(states),
            _ => None,
        })
        .expect("a dictionary of states")
}

/// `style`: Table 192's `/CA` holding the glyph's `ZapfDingbats` code, and both of §12.7.5.2.3's
/// states constructed as the page draws them and written under the names the widget selects them
/// by — so the next reader draws the glyph with no flag asking it to (ADRs 1665, 1676).
#[test]
fn a_style_is_the_check_box_caption_and_its_states_are_written() {
    let document = check_box_document("/ZaDb");
    let mut view = ViewState::of(&document);
    view.run_scripts_with(Some(Arc::new(Setting(vec![Property::Style(Glyph::Cross)]))));
    view.run_open_scripts(&document, 0);
    let written = view.save(&document).expect("the update writes");
    assert!(
        written.unconstructed.is_empty(),
        "every state was written: {:?}",
        written.unconstructed
    );
    let saved = Document::open(written.bytes).expect("the update reads back");
    let widget = object(&saved, 4);
    assert_eq!(
        saved
            .get_key(&characteristics(&saved, &widget), "CA")
            .as_string()
            .map(<[u8]>::to_vec),
        Some(b"8".to_vec())
    );
    // The on state is the construction's glyph in the `/DA` font, not the producer's `(4)`; the
    // face is subset and embedded, so the code shown is the subset's (ADR 1425).
    let on = state_appearance(&saved, &widget, "Yes");
    assert!(
        on.contains("/ZaDb") && on.contains(" Tj") && !on.contains("(4)"),
        "{on}"
    );
    let off = state_appearance(&saved, &widget, "Off");
    assert!(!off.contains("Tj"), "the off state draws no glyph: {off}");
    // New objects, because the producer's states may be other widgets' too.
    let states = normal_states(&saved, &widget);
    for (state, producer) in [("Yes", 6), ("Off", 7)] {
        assert!(
            states
                .get(state)
                .and_then(Object::as_reference)
                .is_some_and(|id| id.number != producer),
            "{state}: {states:?}"
        );
    }
    // Table 170's `/D` defaults to `/N`, and a stored down state would show the producer's glyph
    // the moment the box is pressed, so it names the constructed states too.
    let appearances = saved.get_key(&widget, "AP");
    let down = appearances
        .as_dict()
        .map(|appearances| saved.get_key(appearances, "D"));
    assert_eq!(
        down,
        Some(Object::Dictionary(states)),
        "the down states are the normal ones"
    );
    let catalog = saved.catalog().expect("a /Root");
    let form = saved.get_key(&catalog, "AcroForm");
    assert!(
        form.as_dict()
            .is_some_and(|form| form.get("NeedAppearances").is_none()),
        "nothing is owed: {form:?}"
    );
}

/// A check box with no states and no on value gives the style's on state no name to be written
/// under, so it is owed — and Table 224's flag reaches a form the catalog holds **directly**, which
/// Table 29 permits as much as a reference (ADR 1677).
#[test]
fn a_style_with_no_on_state_is_owed_and_flagged_in_a_direct_form() {
    let [pages, page] = pages("4 0 R");
    let document = assembled(&[
        catalog(),
        pages,
        page,
        "<< /Type /Annot /Subtype /Widget /FT /Btn /T (Field) /V /Off /AS /Off \
         /Rect [10 10 30 30] /F 4 /P 3 0 R /DA (/ZaDb 0 Tf 0 g) /MK << /BG [1] /BC [0] >> >>"
            .to_owned(),
        font(),
    ]);
    assert!(
        document
            .catalog()
            .is_ok_and(|catalog| matches!(catalog.get("AcroForm"), Some(Object::Dictionary(_)))),
        "the fixture's form is direct"
    );
    let mut view = ViewState::of(&document);
    view.run_scripts_with(Some(Arc::new(Setting(vec![Property::Style(Glyph::Star)]))));
    view.run_open_scripts(&document, 0);
    let written = view.save(&document).expect("the update writes");
    assert_eq!(written.unconstructed, vec!["Field".to_owned()]);
    let saved = Document::open(written.bytes).expect("the update reads back");
    let catalog = saved.catalog().expect("a /Root");
    let form = saved.get_key(&catalog, "AcroForm");
    let form = form.as_dict().expect("the form");
    assert!(
        matches!(
            saved.get_key(form, "NeedAppearances"),
            Object::Boolean(true)
        ),
        "{form:?}"
    );
    // The rest of the form is the producer's, and the catalog's other entries survive the rewrite.
    assert!(
        form.get("Fields").is_some() && form.get("DR").is_some(),
        "{form:?}"
    );
    assert!(catalog.get("OpenAction").is_some(), "{catalog:?}");
}

/// Table 224's flag and §12.7.4.3's `/DR` font, both written into one indirect form dictionary.
///
/// Two writers rewrite the form in one update — the free text annotation's font stated in `/DR`,
/// and `/NeedAppearances` for the check box whose on state has no name — and the second keeps the
/// first's entry only by reading the form as the update already has it (ADR 1677).
#[test]
fn the_flag_and_the_free_text_font_are_written_into_one_form() {
    let [pages, page] = pages("4 0 R");
    let document = assembled(&[
        "<< /Type /Catalog /Pages 2 0 R /OpenAction << /S /JavaScript /JS (paint\\(\\);) >> \
         /AcroForm 5 0 R >>"
            .to_owned(),
        pages,
        page,
        "<< /Type /Annot /Subtype /Widget /FT /Btn /T (Field) /V /Off /AS /Off \
         /Rect [10 10 30 30] /F 4 /P 3 0 R /DA (/ZaDb 0 Tf 0 g) /MK << /BC [0] >> >>"
            .to_owned(),
        "<< /Fields [4 0 R] >>".to_owned(),
    ]);
    let mut view = ViewState::of(&document);
    view.run_scripts_with(Some(Arc::new(Setting(vec![Property::Style(Glyph::Check)]))));
    view.run_open_scripts(&document, 0);
    let page = ObjectId {
        number: 3,
        generation: 0,
    };
    view.add_free_text(
        &document,
        page,
        [72.0, 200.0, 300.0, 280.0],
        "note",
        [0.0; 3],
    )
    .expect("a rectangle with area is something to write in");
    let written = view.save(&document).expect("the update writes");
    assert_eq!(written.unconstructed, vec!["Field".to_owned()]);
    let saved = Document::open(written.bytes).expect("the update reads back");
    let form = object(&saved, 5);
    assert!(
        matches!(
            saved.get_key(&form, "NeedAppearances"),
            Object::Boolean(true)
        ),
        "{form:?}"
    );
    let resources = saved.get_key(&form, "DR");
    let fonts = resources
        .as_dict()
        .map(|resources| saved.get_key(resources, "Font"));
    assert!(
        fonts
            .as_ref()
            .and_then(Object::as_dict)
            .is_some_and(|fonts| fonts.get("Helv").is_some()),
        "the free text's font survives the flag: {form:?}"
    );
}

/// In a `/DA` font other than `ZapfDingbats` the style's code is a letter, so the style is not
/// drawn and the reader is told (ADR 1665).
#[test]
fn a_style_in_another_font_is_reported_and_not_drawn() {
    let document = check_box_document("/Helv");
    let mut view = ViewState::of(&document);
    view.run_scripts_with(Some(Arc::new(Setting(vec![Property::Style(Glyph::Star)]))));
    view.run_open_scripts(&document, 0);
    assert!(
        view.script_reports()
            .iter()
            .any(|sentence| sentence.contains("not ZapfDingbats")),
        "{:?}",
        view.script_reports()
    );
    assert!(view.script_properties("Field").is_empty());
}
