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
fn fixture(fields: &[(&str, &str)], config: Option<&str>) -> Document {
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
    // A `config` packet goes after the widgets, so that their numbers stay 8 onwards.
    let config_packet = config
        .map(|_| format!(" (config) {} 0 R", 8 + fields.len()))
        .unwrap_or_default();
    let mut objects = vec![
        format!(
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] \
             /XFA [(template) 5 0 R (datasets) 6 0 R (localeSet) 7 0 R{config_packet}] >> >>"
        ),
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
    if let Some(config) = config {
        objects.push(stream(config));
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
    let document = fixture(fields, None);
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

/// A `bind` picture writes the data in its own format (XFA 3.3 page 176), and a `validate`
/// picture's format test passes what it can format (page 382), in the field's prevailing locale,
/// `nl_NL`, whose radix is `,` and whose grouping is `.` (ADR 2434):
///
/// - `paid`, a `decimal` with no edit picture and the bind picture `zzz9.99`: what was typed is
///   parsed by the bind picture (Rule 2, page 163), and the data is the canonical `12.50` written
///   out by it;
/// - `born`, a `date` typed through `DD-MM-YYYY` and bound by `DD.MM.YYYY`;
/// - `memo`, text whose bind picture is numeric and cannot write it: written as it is;
/// - `checked`, an `integer` whose validate picture `9999` formats `12` as `0012`, so it passes;
/// - `code`, text whose validate picture `AAA-999` formats `ABC123`;
/// - `loose`, whose validate picture would refuse its value, with `formatTest="disabled"`.
#[test]
fn a_bind_and_a_validate_picture_write_their_data() {
    let fields = [
        (
            "paid",
            "<field name=\"paid\"><ui><numericEdit/></ui><bind><picture>zzz9.99</picture></bind>\
             <value><decimal/></value></field>",
        ),
        (
            "born",
            "<field name=\"born\"><ui><picture>DD-MM-YYYY</picture><dateTimeEdit/></ui>\
             <bind><picture>DD.MM.YYYY</picture></bind><value><date/></value></field>",
        ),
        (
            "memo",
            "<field name=\"memo\"><ui><textEdit/></ui><bind><picture>num{zz9}</picture></bind>\
             </field>",
        ),
        (
            "checked",
            "<field name=\"checked\"><ui><numericEdit/></ui><validate><picture>9999</picture>\
             </validate><value><integer/></value></field>",
        ),
        (
            "code",
            "<field name=\"code\"><ui><textEdit/></ui><validate formatTest=\"error\">\
             <picture>AAA-999</picture></validate><value><text/></value></field>",
        ),
        (
            "loose",
            "<field name=\"loose\"><ui><numericEdit/></ui><validate formatTest=\"disabled\">\
             <picture>99</picture></validate><value><integer/></value></field>",
        ),
    ];
    let (values, unkept) = saved(
        &fields,
        &[
            ("paid", "12,50"),
            ("born", "26-10-2026"),
            ("memo", "binnenkort"),
            ("checked", "12"),
            ("code", "ABC123"),
            ("loose", "12345"),
        ],
    );
    assert!(unkept.is_empty(), "{unkept:?}");
    let expected = ["12,50", "26.10.2026", "binnenkort", "12", "ABC123", "12345"];
    for (value, expected) in values.iter().zip(expected) {
        assert_eq!(value.as_deref(), Some(expected));
    }
}

/// What is not written is named, each with its sentence, and its data value is left as it was:
/// non-numeric text in a `numericEdit` (page 156 would make it zero), a value its `validate`
/// picture's format test fails at `error` and at `warning`, page 880's default (pages 162, 382
/// and 385), a date-time typed with no picture, and a time zone in a picture (page 1176).
#[test]
fn a_typed_field_this_cannot_write_is_named() {
    let fields = [
        (
            "total",
            "<field name=\"total\"><ui><numericEdit/></ui><value><integer/></value></field>",
        ),
        (
            "refused",
            "<field name=\"refused\"><ui><numericEdit/></ui><validate formatTest=\"error\">\
             <picture>99</picture></validate><value><integer/></value></field>",
        ),
        (
            "warned",
            "<field name=\"warned\"><ui><numericEdit/></ui><validate><picture>99</picture>\
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
        ("refused", "123"),
        ("warned", "456"),
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

/// XFA 3.3's null test at a save, on a form whose `config` asks for its validations before one
/// (page 1340's `preSave`): page 381's first test, page 384's two levels, page 383's message
/// (EXAMPLE 10.5's), and page 881's default, `disabled`.
///
/// Each field the person emptied is reported, and the save is not refused — page 1340 has a
/// processor save the form whether its validations succeed or fail — so the empty value is the
/// data's as it is the `/V`'s, and nothing is unkept (ADR 2450). The same form configured to
/// validate before submitting alone, page 1340's default, is saved with no report.
#[test]
fn a_save_reports_each_field_its_null_test_holds_and_writes_it_empty() {
    let fields = [
        (
            "amount",
            "<field name=\"amount\"><ui><textEdit/></ui><validate nullTest=\"error\">\
             <message><text name=\"nullTest\">Enter the amount of the loan in dollars.</text>\
             </message></validate></field>",
        ),
        (
            "remark",
            "<field name=\"remark\"><ui><textEdit/></ui><validate nullTest=\"warning\"/></field>",
        ),
        (
            "free",
            "<field name=\"free\"><ui><textEdit/></ui><validate/></field>",
        ),
    ];
    let typed = [("amount", ""), ("remark", ""), ("free", "")];
    let reported = |validate: &str| {
        let config = format!(
            "<config xmlns=\"http://www.xfa.org/schema/xci/3.1/\"><acrobat>\
             <validate>{validate}</validate></acrobat></config>"
        );
        let document = fixture(&fields, Some(&config));
        let mut view = ViewState::of(&document);
        for (name, value) in typed {
            let name = format!("form1[0].{name}[0]");
            assert!(
                view.set_field(&document, &name, &Entered::Text(value.to_owned())) > 0,
                "{name} took no value"
            );
        }
        let written = view.save(&document).expect("the fixture saves");
        assert!(written.xfa_unkept.is_empty(), "{:?}", written.xfa_unkept);
        let saved = Document::open(written.bytes).expect("the save opens");
        let after = Resource::read(&saved)
            .expect("the resource is still stated")
            .expect("the written packet reads");
        for (name, _) in typed {
            assert_eq!(
                after
                    .datasets
                    .value(&after.template, &format!("form1[0].{name}[0]"))
                    .as_deref(),
                Some(""),
                "{name}"
            );
        }
        written.xfa_invalid
    };
    let said = reported("preSubmit preSave");
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(
        said[0][0].starts_with("form1[0].amount[0] is saved empty, and its XFA form requires")
            && said[0][0].contains("\"Enter the amount of the loan in dollars.\""),
        "{said:?}"
    );
    assert!(
        said[1][0].starts_with("form1[0].remark[0] is saved empty, and its XFA form recommends"),
        "{said:?}"
    );
    assert!(reported("preSubmit").is_empty());
}

/// A submit-form action over a fixed URL and no flags, Table 239 read from a one-object document.
fn submit_action() -> pdf_model::action::SubmitForm {
    let source = "%PDF-2.0\n1 0 obj\n<< /Type /Catalog >>\nendobj\n\
                  2 0 obj\n<< /S /SubmitForm /F << /FS /URL /F (https://example.invalid/cgi) >> \
                  /Flags 0 >>\nendobj\ntrailer\n<< /Root 1 0 R /Size 3 >>\n";
    let holder = Document::open(pdf_syntax::FileBytes::from(source.as_bytes().to_vec()))
        .expect("the action parses");
    let read =
        pdf_model::action::read(&holder, &Object::Reference(pdf_syntax::ObjectId::new(2, 0)));
    read.into_iter()
        .find_map(|action| match action {
            pdf_model::action::Action::SubmitForm(submit) => Some(submit),
            _ => None,
        })
        .expect("Table 239's action is read whole")
}

/// What a submit of the fixture, with `typed` entered, says it owes (`Submission::owed`).
fn owed_at_submit(
    fields: &[(&str, &str)],
    config: Option<&str>,
    typed: &[(&str, &str)],
) -> Vec<String> {
    let document = fixture(fields, config);
    let mut view = ViewState::of(&document);
    for (name, value) in typed {
        let name = format!("form1[0].{name}[0]");
        assert!(
            view.set_field(&document, &name, &Entered::Text((*value).to_owned())) > 0,
            "{name} took no value"
        );
    }
    pdf_model::submission::compose(&document, &view, &submit_action(), None)
        .expect("the fixture composes")
        .owed
}

/// XFA 3.3 pages 381 and 386's datatype test and the null test, run before a submit as page 1340's
/// default, `preSubmit`, asks (ADR 2475): an integer field holding no integer and a required field
/// left empty are named on the submission, which is composed whatever is found, and a form that
/// supplies an empty `validate` element is not tested at all.
#[test]
fn a_submit_names_the_validations_the_form_fails_and_still_composes() {
    let fields = [
        (
            "count",
            "<field name=\"count\"><ui><textEdit/></ui><value><integer/></value></field>",
        ),
        (
            "amount",
            "<field name=\"amount\"><ui><textEdit/></ui><validate nullTest=\"error\">\
             <message><text name=\"nullTest\">Enter the amount.</text></message></validate>\
             </field>",
        ),
        (
            "free",
            "<field name=\"free\"><ui><textEdit/></ui><value><text/></value></field>",
        ),
    ];
    let typed = [("count", "abc"), ("amount", ""), ("free", "anything")];
    let owed = owed_at_submit(&fields, None, &typed);
    assert_eq!(owed.len(), 2, "{owed:?}");
    let count = owed
        .iter()
        .find(|said| said.starts_with("form1[0].count[0] holds \"abc\""))
        .expect("the datatype test names the integer field");
    assert!(
        count.contains("is no integer") && count.contains("before a submit (page 1340)"),
        "{count}"
    );
    let amount = owed
        .iter()
        .find(|said| said.starts_with("form1[0].amount[0] is submitted empty"))
        .expect("the null test names the required field");
    assert!(amount.contains("\"Enter the amount.\""), "{amount}");
    let config = "<config xmlns=\"http://www.xfa.org/schema/xci/3.1/\"><acrobat><validate/>\
                  </acrobat></config>";
    assert!(owed_at_submit(&fields, Some(config), &typed).is_empty());
}

/// XFA 3.3 page 527's default bind picture: a field whose template states no bind picture is
/// written by the one a `config` packet's `transform` gives the data element it binds, named by
/// the transform's `ref` — here a `decimal` typed in `nl_NL`, whose canonical `12.50` (chapter
/// 24) the picture `zz9.99` writes in that locale's symbols, its radix a comma. A `transform`
/// naming another element leaves the field canonical (ADR 2450).
#[test]
fn a_config_transform_gives_a_field_its_default_bind_picture() {
    let fields = [(
        "prix",
        "<field name=\"prix\"><ui><numericEdit/></ui><value><decimal/></value></field>",
    )];
    let written = |reference: &str| {
        let config = format!(
            "<config xmlns=\"http://www.xfa.org/schema/xci/3.1/\"><acrobat><common><data>\
             <transform ref=\"{reference}\"><picture>zz9.99</picture></transform></data>\
             </common></acrobat></config>"
        );
        let document = fixture(&fields, Some(&config));
        let mut view = ViewState::of(&document);
        assert!(
            view.set_field(
                &document,
                "form1[0].prix[0]",
                &Entered::Text("12,50".to_owned())
            ) > 0
        );
        let written = view.save(&document).expect("the fixture saves");
        assert!(written.xfa_unkept.is_empty(), "{:?}", written.xfa_unkept);
        let saved = Document::open(written.bytes).expect("the save opens");
        let after = Resource::read(&saved)
            .expect("the resource is still stated")
            .expect("the written packet reads");
        after.datasets.value(&after.template, "form1[0].prix[0]")
    };
    assert_eq!(written("prix").as_deref(), Some("12,50"));
    assert_eq!(written("price").as_deref(), Some("12.50"));
}

/// An extended mapping rule this program does not carry out — page 509's table's
/// `startNode`, here — changes which data node a field binds, so the save names the field rather
/// than binding past the rule (trap 5; ADR 2450).
#[test]
fn a_mapping_rule_this_does_not_carry_out_leaves_the_field_named() {
    let fields = [("note", "<field name=\"note\"><ui><textEdit/></ui></field>")];
    let config = "<config xmlns=\"http://www.xfa.org/schema/xci/3.1/\"><present><common><data>\
                  <startNode>xfa.datasets.data.form1</startNode></data></common></present></config>";
    let document = fixture(&fields, Some(config));
    let mut view = ViewState::of(&document);
    assert!(
        view.set_field(
            &document,
            "form1[0].note[0]",
            &Entered::Text("x".to_owned())
        ) > 0
    );
    let written = view.save(&document).expect("the fixture saves");
    assert_eq!(written.xfa_unkept.len(), 1, "{:?}", written.xfa_unkept);
    assert!(
        written.xfa_unkept[0].1.contains("startNode"),
        "{:?}",
        written.xfa_unkept
    );
}

/// The calculations and the form they run in (RFC 0011 section 5's step 6, ADR 2482): the scripts
/// in document order with their language (XFA 3.3 page 412), the names a script reaches by page
/// 114's search and page 115's inferred index, and the values it reads and assigns.
mod calculations {
    use pdf_model::xfa::{Change, Language, Occurrence, Resource, Scalar, Step};
    use pdf_syntax::Document;

    use super::fixture;

    fn step(name: &str) -> Step {
        Step {
            name: name.to_owned(),
            occurrence: Occurrence::Same,
        }
    }

    fn at(name: &str, index: i64) -> Step {
        Step {
            name: name.to_owned(),
            occurrence: Occurrence::At(index),
        }
    }

    fn resource(fields: &[(&str, &str)]) -> Resource {
        Resource::read(&fixture(fields, None))
            .expect("the fixture states an XFA resource")
            .expect("its resource reads")
    }

    #[test]
    fn scripts_are_listed_in_document_order_with_their_language() {
        let resource = resource(&[
            (
                "Units",
                "<field name=\"Units\"><value><integer/></value></field>",
            ),
            (
                "Total",
                "<field name=\"Total\"><calculate><script>Units * 2</script></calculate></field>",
            ),
            (
                "Script",
                "<field name=\"Script\"><calculate><script \
                 contentType=\"application/x-javascript\">this.rawValue = 1;</script>\
                 </calculate></field>",
            ),
            (
                "Blank",
                "<field name=\"Blank\"><calculate><script>  </script></calculate></field>",
            ),
        ]);
        let form = resource.form(&[("form1[0].Total[0]".to_owned(), Change::Text(String::new()))]);
        let listed = form.calculations();
        assert_eq!(listed.len(), 2, "a blank script is no calculation");
        assert_eq!(listed[0].language, Language::FormCalc);
        assert_eq!(listed[0].script, "Units * 2");
        assert_eq!(listed[0].field.as_deref(), Some("form1[0].Total[0]"));
        assert!(listed[0].sets_value);
        assert_eq!(listed[1].language, Language::JavaScript);
        assert_eq!(listed[1].field, None);
    }

    #[test]
    fn a_name_is_searched_from_the_children_up_through_the_siblings_of_each_ancestor() {
        // Page 114: children first, then the container and its siblings, then the parent and its
        // siblings, so an inner field shadows an outer one of the same name and a name only an
        // outer container holds is still found.
        let resource = resource(&[
            ("Units", "<field name=\"Units\"/>"),
            (
                "Group",
                "<subform name=\"Group\"><field name=\"Units\"/>\
                 <field name=\"Inner\"><calculate><script>Units</script></calculate></field>\
                 <field name=\"Other\"><calculate><script>Total</script></calculate></field>\
                 </subform>",
            ),
            ("Total", "<field name=\"Total\"/>"),
        ]);
        let form = resource.form(&[]);
        let listed = form.calculations();
        let (inner, other) = (listed[0].node, listed[1].node);
        let outer_units = form.resolve(other, &[step("Units")]).unwrap();
        let inner_units = form.resolve(inner, &[step("Units")]).unwrap();
        assert_eq!(outer_units.len(), 1);
        assert_eq!(inner_units.len(), 1);
        assert_eq!(
            outer_units, inner_units,
            "both reach the Units beside them, in Group"
        );
        let top_units = form.resolve(inner, &[at("Units", 0)]).unwrap();
        assert_eq!(top_units, inner_units);
        let total = form.resolve(other, &[step("Total")]).unwrap();
        assert_eq!(
            total.len(),
            1,
            "a name only the parent's sibling holds is found"
        );
        assert_eq!(form.resolve(inner, &[step("Missing")]).unwrap(), vec![]);
        assert_eq!(
            form.resolve(inner, &[step("Group"), step("Inner")])
                .unwrap(),
            vec![inner],
            "a name's later terms are children of the one before"
        );
    }

    #[test]
    fn a_dollar_names_the_script_container_and_escapes_the_search() {
        // Page 114: `$.` is qualified and relative to the container, so it does not find the
        // same-named field a level up.
        let resource = resource(&[
            ("Price", "<field name=\"Price\"/>"),
            (
                "Detail",
                "<subform name=\"Detail\"><field name=\"Cost\"><calculate>\
                 <script>$.Price</script></calculate></field></subform>",
            ),
        ]);
        let form = resource.form(&[]);
        let cost = form.calculations()[0].node;
        assert_eq!(form.resolve(cost, &[step("$")]).unwrap(), vec![cost]);
        assert_eq!(
            form.resolve(cost, &[step("$"), step("Price")]).unwrap(),
            vec![]
        );
        assert_eq!(form.resolve(cost, &[step("Price")]).unwrap().len(), 1);
        assert!(
            form.resolve(cost, &[step("$record"), step("Price")])
                .is_err()
        );
        assert!(form.resolve(cost, &[step("$"), step("parent")]).is_err());
    }

    #[test]
    fn an_omitted_index_is_the_index_of_the_script_container() {
        // Page 115 (Example 3.39): in a matrix of same-named fields, Total[n] reads Units[n].
        let resource = resource(&[(
            "Sheet",
            "<subform name=\"Sheet\"><field name=\"Units\"/>\
             <field name=\"Total\"><calculate><script>Units</script></calculate></field>\
             <field name=\"Units\"/>\
             <field name=\"Total\"><calculate><script>Units</script></calculate></field>\
             </subform>",
        )]);
        let form = resource.form(&[]);
        let listed = form.calculations();
        let (first, second) = (listed[0].node, listed[1].node);
        let units = |here, index| form.resolve(here, &[at("Units", index)]).unwrap();
        assert_eq!(
            form.resolve(first, &[step("Units")]).unwrap(),
            units(first, 0)
        );
        assert_eq!(
            form.resolve(second, &[step("Units")]).unwrap(),
            units(second, 1)
        );
        assert_ne!(units(first, 0), units(first, 1));
        let relative = Step {
            name: "Units".to_owned(),
            occurrence: Occurrence::Relative(-1),
        };
        assert_eq!(form.resolve(second, &[relative]).unwrap(), units(second, 0));
        assert_eq!(
            form.resolve(first, &[at("Units", 5)]).unwrap(),
            vec![],
            "an index outside the range names nothing"
        );
    }

    #[test]
    fn values_are_typed_by_the_container_and_assignments_are_named() {
        let resource = resource(&[
            (
                "Units",
                "<field name=\"Units\"><value><integer/></value></field>",
            ),
            (
                "Note",
                "<field name=\"Note\"><value><text/></value></field>",
            ),
            (
                "Total",
                "<field name=\"Total\"><value><decimal/></value><calculate>\
                 <script>Units</script></calculate></field>",
            ),
            ("Group", "<subform name=\"Group\"/>"),
        ]);
        let mut form = resource.form(&[
            ("form1[0].Units[0]".to_owned(), Change::Text("3".to_owned())),
            ("form1[0].Note[0]".to_owned(), Change::Text("3".to_owned())),
            ("form1[0].Total[0]".to_owned(), Change::Text(String::new())),
        ]);
        let total = form.calculations()[0].node;
        let units = form.resolve(total, &[step("Units")]).unwrap()[0];
        let note = form.resolve(total, &[step("Note")]).unwrap()[0];
        let group = form.resolve(total, &[step("Group")]).unwrap()[0];
        assert_eq!(form.value(units), Scalar::Number(3.0));
        assert_eq!(form.value(note), Scalar::Text("3".to_owned()));
        assert_eq!(form.value(total), Scalar::Null);
        assert!(form.edits().is_empty());
        form.set(total, Scalar::Number(6.0)).unwrap();
        assert!(
            form.set(group, Scalar::Null).is_err(),
            "a subform holds no value"
        );
        assert_eq!(
            form.edits(),
            vec![("form1[0].Total[0]".to_owned(), Scalar::Number(6.0))]
        );
    }

    /// A document whose `/XFA` entry is one stream holding the whole `xdp:xdp` (§K.2's first form).
    fn whole_resource(xdp: &str) -> Document {
        use std::fmt::Write as _;
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [] /XFA 4 0 R >> >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>".to_owned(),
            format!(
                "<< /Length {} >>\nstream\n{xdp}\nendstream",
                xdp.len().saturating_add(1)
            ),
        ];
        let mut out = String::from("%PDF-1.7\n");
        let mut offsets = Vec::new();
        for (index, body) in objects.iter().enumerate() {
            offsets.push(out.len());
            let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
        }
        let xref_at = out.len();
        let _ = write!(
            out,
            "xref\n0 {}\n0000000000 65535 f \n",
            objects.len().saturating_add(1)
        );
        for offset in &offsets {
            let _ = writeln!(out, "{offset:010} 00000 n ");
        }
        let _ = write!(
            out,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
            objects.len().saturating_add(1)
        );
        Document::open(out.into_bytes()).expect("the fixture opens")
    }

    #[test]
    fn a_configs_template_element_is_not_the_forms_template() {
        // Page 1335: the configuration has a `template` element of its own (a base, a start page,
        // a uri), and in a whole-resource stream it comes before the form's, so the form's is the
        // `xdp:xdp` root's child.
        let document = whole_resource(
            "<xdp:xdp xmlns:xdp=\"http://ns.adobe.com/xdp/\">\
             <config xmlns=\"http://www.xfa.org/schema/xci/1.0/\"><agent name=\"designer\">\
             <template><base/></template></agent></config>\
             <template xmlns=\"http://www.xfa.org/schema/xfa-template/3.3/\">\
             <subform name=\"form1\"><field name=\"Total\"><calculate>\
             <script>1 + 1</script></calculate></field></subform></template>\
             <xfa:datasets xmlns:xfa=\"http://www.xfa.org/schema/xfa-data/1.0/\">\
             <xfa:data><form1/></xfa:data></xfa:datasets></xdp:xdp>",
        );
        let resource = Resource::read(&document)
            .expect("the entry is an XFA resource")
            .expect("its resource reads");
        let form = resource.form(&[]);
        let listed = form.calculations();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].script, "1 + 1");
    }
}
