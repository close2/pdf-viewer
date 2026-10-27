//! §K.2's first writer rule over the corpora's four hybrid forms: filled, saved, read back.
//!
//! > The XFA field values shall be consistent with the corresponding V entries of the PDF field
//! > objects.
//!
//! RFC 0011 section 4's census found four documents in `doc/pdf.js` and `doc/corpora` that carry
//! an XFA resource and do not ask to be regenerated; each is filled here in the fields its
//! template's kinds allow — text, check button, exclusion group, single and multiple choice,
//! a globally bound choice — saved by §7.5.6's update, re-opened, and its `datasets` packet read
//! again. Three things are asserted for each (ADR 2396):
//!
//! 1. the save names no field it could not keep (`Written::xfa_unkept` is empty);
//! 2. every field filled reads back from the data with the value its `/V` was written with,
//!    turned into XFA's terms by the clause the comment beside it names;
//! 3. every field **not** filled reads back from the data as it did before the save — the
//!    producer's values are kept, not re-derived.
//!
//! The documents are pdf.js's test files and are read from its checkout; where it is absent the
//! test says so and passes, as every pdf.js-corpus test in this crate does.

#![expect(
    clippy::expect_used,
    reason = "a test's failure is its purpose, and these helpers run outside #[test] bodies \
              where `allow-panic-in-tests` does not reach"
)]

use std::path::Path;

use pdf_model::view::{Entered, ViewState};
use pdf_model::xfa::Resource;
use pdf_syntax::{Document, Object};

/// One of pdf.js's test documents, or `None` when the checkout is absent.
fn open(name: &str) -> Option<(Vec<u8>, Document)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/pdf.js/test/pdfs")
        .join(name);
    let bytes = std::fs::read(path).ok()?;
    let document = Document::open(bytes.clone()).expect("a corpus document opens");
    Some((bytes, document))
}

/// A field's `/V`, inherited up §12.7.4.1's chain from its first widget, as text.
fn stated_value(document: &Document, name: &str) -> Option<Object> {
    let table = pdf_model::view::widgets_by_field_name(document);
    let mut id = *table.get(name)?.first()?;
    for _ in 0..32 {
        let object = document.get(id);
        let dict = object.as_dict()?;
        let value = document.get_key(dict, "V");
        if !value.is_null() {
            return Some(value);
        }
        id = dict.get("Parent").and_then(Object::as_reference)?;
    }
    None
}

/// Fills `fills`, saves, re-opens, and checks the three assertions; `expected` is each filled
/// field's data value after the save.
fn round_trip(file: &str, fills: &[(&str, Entered, &str)]) {
    let Some((original, document)) = open(file) else {
        eprintln!("{file}: the pdf.js checkout is absent; nothing judged");
        return;
    };
    let before = Resource::read(&document)
        .expect("a hybrid states an XFA resource")
        .expect("its resource reads");
    let names: Vec<String> = pdf_model::view::widgets_by_field_name(&document)
        .into_keys()
        .collect();
    let mut view = ViewState::of(&document);
    for (name, value, _) in fills {
        assert!(
            view.set_field(&document, name, value) > 0,
            "{file}: {name} took no value"
        );
    }
    let written = view.save(&document).expect("the hybrid saves");
    assert!(
        written.xfa_unkept.is_empty(),
        "{file}: {:?}",
        written.xfa_unkept
    );
    assert!(
        written.bytes.starts_with(&original),
        "{file}: §7.5.6 appends"
    );
    if let Ok(out) = std::env::var("XFA_DATASETS_OUT") {
        let _ = std::fs::write(Path::new(&out).join(file), &written.bytes);
    }
    let saved = Document::open(written.bytes).expect("the save opens");
    let after = Resource::read(&saved)
        .expect("the resource is still stated")
        .expect("the written packet reads");
    for (name, _, expected) in fills {
        assert_eq!(
            after.datasets.value(&after.template, name).as_deref(),
            Some(*expected),
            "{file}: {name}'s data value, beside the /V {:?}",
            stated_value(&saved, name)
        );
    }
    for name in names
        .iter()
        .filter(|name| !fills.iter().any(|(f, _, _)| f == name))
    {
        assert_eq!(
            after.datasets.value(&after.template, name),
            before.datasets.value(&before.template, name),
            "{file}: {name} was not filled and its data moved"
        );
    }
}

/// Text fields: the data value is the text (XFA 3.3's `textEdit`), a carriage return — a line
/// break in a `/V` by §12.7.4.3 — and markup characters included.
#[test]
fn annotation_text_widget_keeps_its_text_values() {
    round_trip(
        "annotation-text-widget.pdf",
        &[
            (
                "formulier1[0].#subform[0].Tekstveld2[0]",
                Entered::Text("Fish & <chips>".to_owned()),
                "Fish & <chips>",
            ),
            (
                "formulier1[0].#subform[0].Tekstveld7[0]",
                Entered::Text("one\rtwo".to_owned()),
                "one\rtwo",
            ),
            (
                "formulier1[0].#subform[0].Tekstveld1[0]",
                Entered::Cleared,
                "",
            ),
        ],
    );
}

/// Check buttons take their template `items`' on and off values (`1`, `0`); a radio set — a
/// named `exclGroup` — takes the on value of the button that is on, which Table 230's `/Opt`
/// states as the export value of the widget whose state is on: the second widget of the second
/// group has state `1`, `/Opt [(1) (2)]` makes its export value `2`, and the template's second
/// button's on item is `2`.
#[test]
fn annotation_button_widget_keeps_check_and_radio_values() {
    round_trip(
        "annotation-button-widget.pdf",
        &[
            (
                "formulier1[0].#subform[0].Selectievakje1[0]",
                Entered::Text("1".to_owned()),
                "1",
            ),
            (
                "formulier1[0].#subform[0].Selectievakje3[0]",
                Entered::Text("Off".to_owned()),
                "0",
            ),
            (
                "formulier1[0].#subform[0].LijstKeuzerondje[1]",
                Entered::Text("1".to_owned()),
                "2",
            ),
            (
                "formulier1[0].#subform[0].LijstKeuzerondje[0]",
                Entered::Text("0".to_owned()),
                "1",
            ),
        ],
    );
}

/// A choice's data value is its export value; a multiple-selection list's data is a group of
/// `value` elements, which [`pdf_model::xfa::Datasets::value`] joins by line feeds (XFA 3.3
/// page 198).
#[test]
fn annotation_choice_widget_keeps_single_and_multiple_choices() {
    round_trip(
        "annotation-choice-widget.pdf",
        &[
            (
                "formulier1[0].#subform[0].Vervolgkeuzelijst1[0]",
                Entered::Chosen(vec![4]),
                "Amet",
            ),
            (
                "formulier1[0].#subform[0].Keuzelijst2[0]",
                Entered::Chosen(vec![0, 2, 7]),
                "Lorem\nDolor\nElit",
            ),
            (
                "formulier1[0].#subform[0].Keuzelijst1[0]",
                Entered::Cleared,
                "",
            ),
        ],
    );
}

/// A subform stating `bind match="none"` is passed through, so its fields' data sits under the
/// record — where the producer's packet already put it — and a `match="global"` choice binds the
/// value of its name in the record first (XFA 3.3 chapter 6, pages 246–247).
#[test]
fn bug1815476_keeps_text_and_globally_bound_choices() {
    round_trip(
        "bug1815476.pdf",
        &[
            (
                "topmostSubform[0].Page1[0].EMAIL[0]",
                Entered::Text("someone@example.org".to_owned()),
                "someone@example.org",
            ),
            (
                "topmostSubform[0].Page1[0].Nivel[0]",
                Entered::Chosen(vec![3]),
                "B2",
            ),
            (
                "topmostSubform[0].Page1[0].Seccion[0]",
                Entered::Chosen(vec![2]),
                "Béjar",
            ),
        ],
    );
}

/// A save that changes no field leaves the packet alone: the update holds no replacement for it.
#[test]
fn a_save_that_changes_no_field_writes_no_packet() {
    let Some((_, document)) = open("annotation-text-widget.pdf") else {
        return;
    };
    let resource = Resource::read(&document).expect("stated").expect("reads");
    let written = ViewState::of(&document).save(&document).expect("saves");
    let saved = Document::open(written.bytes).expect("opens");
    assert_eq!(
        saved.get(resource.datasets_stream),
        document.get(resource.datasets_stream)
    );
}

/// A hybrid form built here, its `AcroForm` fields all text fields under `form1[0]`, its XFA
/// resource §K.2's packet array of a template, a `datasets` packet and a `localeSet` packet whose
/// one locale, `nl_NL`, has the symbols and patterns the corpora's `nl_NL` hybrids carry.
///
/// `fields` are each field's name and its template element; every field's data value starts as
/// `-`.
#[expect(
    clippy::arithmetic_side_effects,
    clippy::format_collect,
    reason = "a fixture of a few fields and a few kilobytes, built once; clarity over allocation"
)]
fn fixture(fields: &[(&str, &str)]) -> Document {
    use std::fmt::Write as _;
    let template = format!(
        "<template xmlns=\"http://www.xfa.org/schema/xfa-template/3.3/\">\
         <subform name=\"form1\" locale=\"nl_NL\">{}</subform></template>",
        fields.iter().map(|(_, field)| *field).collect::<String>()
    );
    let datasets = format!(
        "<xfa:datasets xmlns:xfa=\"http://www.xfa.org/schema/xfa-data/1.0/\"><xfa:data><form1>{}\
         </form1></xfa:data></xfa:datasets>",
        fields
            .iter()
            .map(|(name, _)| format!("<{name}>-</{name}>"))
            .collect::<String>()
    );
    let names = |tag: &str, list: &str| {
        list.split(' ')
            .map(|name| format!("<{tag}>{name}</{tag}>"))
            .collect::<String>()
    };
    let locale_set = format!(
        "<localeSet xmlns=\"http://www.xfa.org/schema/xfa-locale-set/2.7/\">\
         <locale name=\"nl_NL\"><calendarSymbols name=\"gregorian\">\
         <monthNames>{}</monthNames><monthNames abbr=\"1\">{}</monthNames>\
         <meridiemNames><meridiem>AM</meridiem><meridiem>PM</meridiem></meridiemNames>\
         </calendarSymbols>\
         <datePatterns><datePattern name=\"med\">D-MMM-YYYY</datePattern></datePatterns>\
         <timePatterns><timePattern name=\"med\">H:MM:SS</timePattern></timePatterns>\
         <numberPatterns><numberPattern name=\"numeric\">z,zz9.zzz</numberPattern></numberPatterns>\
         <numberSymbols><numberSymbol name=\"decimal\">,</numberSymbol>\
         <numberSymbol name=\"grouping\">.</numberSymbol><numberSymbol name=\"percent\">%</numberSymbol>\
         <numberSymbol name=\"minus\">-</numberSymbol><numberSymbol name=\"zero\">0</numberSymbol>\
         </numberSymbols></locale></localeSet>",
        names(
            "month",
            "januari februari maart april mei juni juli augustus september oktober november december"
        ),
        names("month", "jan feb mrt apr mei jun jul aug sep okt nov dec"),
    );
    let widgets: Vec<usize> = (0..fields.len()).map(|i| 8 + i).collect();
    let references = |ids: &[usize]| {
        ids.iter()
            .map(|id| format!("{id} 0 R"))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let stream = |text: &str| {
        format!(
            "<< /Length {} >>\nstream\n{text}\nendstream",
            text.len() + 1
        )
    };
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] \
         /XFA [(template) 5 0 R (datasets) 6 0 R (localeSet) 7 0 R] >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [{}] >>",
            references(&widgets)
        ),
        format!("<< /T (form1[0]) /Kids [{}] >>", references(&widgets)),
        stream(&template),
        stream(&datasets),
        stream(&locale_set),
    ];
    for (index, (name, _)) in fields.iter().enumerate() {
        let y = 700 - 30 * index;
        objects.push(format!(
            "<< /Type /Annot /Subtype /Widget /FT /Tx /Parent 4 0 R /T ({name}[0]) /P 3 0 R \
             /Rect [72 {y} 300 {}] /DA (/Helv 10 Tf 0 g) >>",
            y + 20
        ));
    }
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index + 1);
    }
    let xref_at = out.len();
    let _ = write!(out, "xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
        objects.len() + 1
    );
    Document::open(out.into_bytes()).expect("the fixture opens")
}

/// Fills the fixture's fields, saves, re-reads the `datasets` packet; each field's data value
/// and what the save named unkept.
fn saved(fields: &[(&str, &str)], typed: &[(&str, &str)]) -> (Vec<Option<String>>, Vec<String>) {
    let document = fixture(fields);
    let mut view = ViewState::of(&document);
    for (name, value) in typed {
        let name = format!("form1[0].{name}[0]");
        assert!(
            view.set_field(&document, &name, &Entered::Text((*value).to_owned())) > 0,
            "{name} took no value"
        );
    }
    let written = view.save(&document).expect("the fixture saves");
    let unkept = written
        .xfa_unkept
        .iter()
        .map(|(name, why)| format!("{name}: {why}"))
        .collect();
    let saved = Document::open(written.bytes).expect("the save opens");
    let after = Resource::read(&saved)
        .expect("the resource is still stated")
        .expect("the written packet reads");
    let values = typed
        .iter()
        .map(|(name, _)| {
            after
                .datasets
                .value(&after.template, &format!("form1[0].{name}[0]"))
        })
        .collect();
    (values, unkept)
}

/// A numeric, decimal, float, date, time and date-time field's data is the canonical form XFA
/// 3.3 chapter 24 defines (pages 1045–1047), reached from what a person typed by chapter 4's
/// input parsing (pages 162–164) in the field's prevailing locale, `nl_NL` from its subform
/// (page 1153):
///
/// - `amount`, a `decimal` in a `numericEdit` with no picture: Rule 2A's default, the locale's
///   `numeric` pattern `z,zz9.zzz` read with `,` as the radix and `.` as the grouping;
/// - `count`, an `integer` with the edit picture `z,zz9`;
/// - `ratio`, a `float` with the edit picture `zz9.99`, whose `.` is the locale's radix;
/// - `born`, a `date` with the edit picture `DD-MM-YYYY`;
/// - `due`, a `date` with no picture: Rule 2A's `med` pattern `D-MMM-YYYY` and the locale's
///   abbreviated month names;
/// - `start`, a `time` with the edit picture `H.MM 'uur'`;
/// - `stamp`, a `dateTime` with page 1157's compound picture of a date clause and a time clause;
/// - `note`, a `date` whose text matches no picture: Rule 2 writes it as typed (page 163).
#[test]
fn a_typed_field_writes_its_canonical_value() {
    let fields = [
        (
            "amount",
            "<field name=\"amount\"><ui><numericEdit/></ui><value><decimal/></value></field>",
        ),
        (
            "count",
            "<field name=\"count\"><ui><picture>z,zz9</picture><numericEdit/></ui>\
             <value><integer/></value></field>",
        ),
        (
            "ratio",
            "<field name=\"ratio\"><ui><picture>zz9.99</picture><numericEdit/></ui>\
             <value><float/></value></field>",
        ),
        (
            "born",
            "<field name=\"born\"><ui><picture>DD-MM-YYYY</picture><dateTimeEdit/></ui>\
             <value><date/></value></field>",
        ),
        (
            "due",
            "<field name=\"due\"><ui><dateTimeEdit/></ui><value><date/></value></field>",
        ),
        (
            "start",
            "<field name=\"start\"><ui><picture>H.MM 'uur'</picture><dateTimeEdit/></ui>\
             <value><time/></value></field>",
        ),
        (
            "stamp",
            "<field name=\"stamp\"><ui><picture>date{DD-MM-YYYY} time{HH:MM}</picture>\
             <dateTimeEdit/></ui><value><dateTime/></value></field>",
        ),
        (
            "note",
            "<field name=\"note\"><ui><picture>DD-MM-YYYY</picture><dateTimeEdit/></ui>\
             <value><date/></value></field>",
        ),
    ];
    let (values, unkept) = saved(
        &fields,
        &[
            ("amount", "1.234,5"),
            ("count", "1.234"),
            ("ratio", "12,50"),
            ("born", "26-10-2026"),
            ("due", "3-nov-2026"),
            ("start", "9.05 uur"),
            ("stamp", "26-10-2026 14:30"),
            ("note", "binnenkort"),
        ],
    );
    assert!(unkept.is_empty(), "{unkept:?}");
    let expected = [
        "1234.5",
        "1234",
        "12.50",
        "2026-10-26",
        "2026-11-03",
        "09:05",
        "2026-10-26T14:30",
        "binnenkort",
    ];
    for (value, expected) in values.iter().zip(expected) {
        assert_eq!(value.as_deref(), Some(expected));
    }
}

/// What is not written is named, each with its sentence, and its data value is left as it was:
/// non-numeric text in a `numericEdit` (page 156 would make it zero), a `bind` picture's
/// localised data (page 161), a `validate` picture's format test (pages 162 and 382), a
/// date-time typed with no picture, and a time zone in a picture (page 1176).
#[test]
fn a_typed_field_this_cannot_write_is_named() {
    let fields = [
        (
            "total",
            "<field name=\"total\"><ui><numericEdit/></ui><value><integer/></value></field>",
        ),
        (
            "paid",
            "<field name=\"paid\"><ui><numericEdit/></ui><bind><picture>zzz9.99</picture></bind>\
             <value><decimal/></value></field>",
        ),
        (
            "checked",
            "<field name=\"checked\"><ui><numericEdit/></ui><validate><picture>9999</picture>\
             </validate><value><integer/></value></field>",
        ),
        (
            "when",
            "<field name=\"when\"><ui><dateTimeEdit/></ui><value><dateTime/></value></field>",
        ),
        (
            "zoned",
            "<field name=\"zoned\"><ui><picture>HH:MM Z</picture><dateTimeEdit/></ui>\
             <value><time/></value></field>",
        ),
    ];
    let typed = [
        ("total", "veel"),
        ("paid", "12"),
        ("checked", "12"),
        ("when", "26-10-2026 14:30"),
        ("zoned", "14:30 CET"),
    ];
    let (values, unkept) = saved(&fields, &typed);
    assert_eq!(unkept.len(), typed.len(), "{unkept:?}");
    for (name, _) in &typed {
        let name = format!("form1[0].{name}[0]: ");
        assert!(
            unkept.iter().any(|line| line.starts_with(&name)),
            "{name} {unkept:?}"
        );
    }
    assert!(
        values.iter().all(|v| v.as_deref() == Some("-")),
        "{values:?}"
    );
}
