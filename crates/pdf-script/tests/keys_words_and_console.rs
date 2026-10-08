//! The members ADR 1762 bridges through the view state: `event.shift`, `modifier` and `keyDown`
//! from the keys a host says are held, a field's `lineWidth`, `textSize` and `textFont` read from
//! Table 168's `/W` and Table 228's `/DA` and written back as entries the save carries, the
//! `console` requests a host takes, the page's words, and `OCG.getIntent`.
//!
//! Every expected value is the clause the member reads — Table 96's `/Intent`, Table 168's `/W`,
//! the `/DA`'s `Tf` — or this program's documented choice, stated in ADR 1762: what a word is, and
//! what `bStrip` removes.

#![cfg(feature = "engine")]
#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use std::fmt::Write as _;

use pdf_model::action::Trigger;
use pdf_model::view::{ConsoleCommand, Entered, Keys, ViewState};
use pdf_script::{Budget, Engine};
use pdf_syntax::{Document, Object, ObjectId};

/// A one-page document: the catalog states `catalog` beside `/Pages`, the page states `page`,
/// lists annotation 4, and `objects` are numbered from 4 — each a dictionary, or a dictionary and
/// the stream data after ` @stream `.
fn document(catalog: &str, page: &str, objects: &[&str]) -> Document {
    let mut bodies = vec![
        format!("<< /Type /Catalog /Pages 2 0 R {catalog} >>"),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [4 0 R] {page} >>"),
    ];
    for object in objects {
        bodies.push(match object.split_once(" @stream ") {
            None => (*object).to_owned(),
            Some((dictionary, data)) => format!(
                "{} /Length {} >>\nstream\n{data}\nendstream",
                dictionary.trim_end_matches(">>"),
                data.len()
            ),
        });
    }
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = bodies.len().saturating_add(1);
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

/// A text field `Out` as object 4, whose mouse-up runs `script` and which states `extra`.
fn out_field(script: &str, extra: &str) -> String {
    format!(
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Out) /Rect [10 10 210 40] /F 4 /P 3 0 R \
         /DA (/Helv 10 Tf 0 g) /AA << /U << /S /JavaScript /JS ({script}) >> >> {extra} >>"
    )
}

/// The engine's view state of `document`, its open sequence run.
fn opened(document: &Document) -> ViewState {
    let mut view = ViewState::of(document);
    view.run_scripts_with(Some(Engine::runner(Budget::FIELD_EVENT)));
    view.run_open_scripts(document, 0);
    view
}

/// Runs the field's mouse-up, and answers what `Out` holds afterwards.
fn press(view: &mut ViewState, document: &Document) -> String {
    let field = ObjectId {
        number: 4,
        generation: 0,
    };
    let ran = view.run_annotation_scripts(document, field, Trigger::Up);
    assert_eq!(ran.handed, 1, "{:?}", view.script_reports());
    view.field_value(document, "Out")
        .map(|shown| shown.text)
        .unwrap_or_default()
}

#[test]
fn a_script_reads_the_keys_the_host_said_were_held() {
    let document = document(
        "/AcroForm << /Fields [4 0 R] >>",
        "",
        &[&out_field(
            "event.target.value = [event.shift, event.modifier, event.keyDown].join\\(\\);",
            "/V ()",
        )],
    );
    let mut view = opened(&document);
    assert_eq!(press(&mut view, &document), "false,false,false");
    view.set_keys(Keys {
        shift: true,
        modifier: true,
        arrows: true,
    });
    // `keyDown` is a choice field's keystroke's alone, so a mouse-up reads it false.
    assert_eq!(press(&mut view, &document), "true,true,false");
}

#[test]
fn the_typographic_three_are_read_from_the_widget_and_written_as_entries_the_save_carries() {
    let document = document(
        "/AcroForm << /Fields [4 0 R] /DR << /Font << /Helv 5 0 R /TiRo 6 0 R >> >> >>",
        "",
        &[
            &out_field(
                "var f = event.target; var was = [f.lineWidth, f.textSize, f.textFont].join\\('|'\\); \
                 f.lineWidth = 3; f.textSize = 14; f.textFont = font.Times; \
                 f.value = was + '>' + [f.lineWidth, f.textSize, f.textFont].join\\('|'\\);",
                "/V () /BS << /W 2 >>",
            ),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
            "<< /Type /Font /Subtype /Type1 /BaseFont /Times-Roman >>",
        ],
    );
    let mut view = opened(&document);
    assert_eq!(
        press(&mut view, &document),
        "2|10|Helvetica>3|14|Times-Roman"
    );
    let written = view.save(&document).expect("the update writes");
    let saved = Document::open(written.bytes).expect("the update reads back");
    let widget = saved
        .get(ObjectId {
            number: 4,
            generation: 0,
        })
        .as_dict()
        .cloned()
        .expect("the widget");
    let Object::String(appearance) = saved.get_key(&widget, "DA") else {
        panic!("the saved widget states a /DA");
    };
    let appearance = String::from_utf8_lossy(&appearance).into_owned();
    assert!(
        appearance.ends_with("/TiRo 14 Tf"),
        "the size and the font are one Tf after the producer's: {appearance}"
    );
    let style = saved.get_key(&widget, "BS");
    let width = style
        .as_dict()
        .and_then(|style| saved.get_key(style, "W").as_number());
    assert_eq!(width, Some(3.0));
}

#[test]
fn a_font_the_form_does_not_hold_is_reported_and_not_written() {
    let document = document(
        "/AcroForm << /Fields [4 0 R] /DR << /Font << /Helv 5 0 R >> >> >>",
        "",
        &[
            &out_field("event.target.textFont = 'Viva-Regular';", "/V ()"),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        ],
    );
    let mut view = opened(&document);
    press(&mut view, &document);
    assert!(
        view.script_reports()
            .iter()
            .any(|sentence| sentence.contains("Viva-Regular") && sentence.contains("/DR")),
        "{:?}",
        view.script_reports()
    );
}

#[test]
fn the_console_s_requests_reach_the_host_in_order_and_a_clear_drops_what_came_before() {
    let document = document(
        "/AcroForm << /Fields [4 0 R] >>",
        "",
        &[&out_field(
            "console.println\\('before'\\); console.clear\\(\\); console.println\\('after'\\); \
             console.show\\(\\);",
            "/V ()",
        )],
    );
    let mut view = opened(&document);
    press(&mut view, &document);
    let requests = view.take_console_requests();
    let commands: Vec<ConsoleCommand> = requests.iter().map(|request| request.command).collect();
    assert_eq!(commands, [ConsoleCommand::Clear, ConsoleCommand::Show]);
    let logged: Vec<&String> = view
        .script_reports()
        .iter()
        .filter(|sentence| sentence.contains("logged"))
        .collect();
    assert!(
        logged.iter().any(|sentence| sentence.contains("after"))
            && !logged.iter().any(|sentence| sentence.contains("before")),
        "{logged:?}"
    );
    assert!(view.take_console_requests().is_empty(), "taken once");
}

#[test]
fn the_word_pair_reads_the_page_s_words() {
    let document = document(
        "/AcroForm << /Fields [4 0 R] >>",
        "/Contents 5 0 R /Resources << /Font << /F1 6 0 R >> >>",
        &[
            &out_field(
                "event.target.value = [this.getPageNumWords\\(0\\), this.getPageNthWord\\(0, 0\\), \
                 this.getPageNthWord\\(0, 0, false\\), this.getPageNthWord\\(0, 2\\)].join\\('|'\\);",
                "/V ()",
            ),
            "<< >> @stream BT /F1 12 Tf 20 300 Td (Hello, wide world.) Tj ET",
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        ],
    );
    let mut view = opened(&document);
    assert_eq!(press(&mut view, &document), "3|Hello|Hello,|world");
}

#[test]
fn a_word_the_page_does_not_have_is_a_range_error() {
    let document = document(
        "/AcroForm << /Fields [4 0 R] >>",
        "/Contents 5 0 R /Resources << /Font << /F1 6 0 R >> >>",
        &[
            &out_field(
                "try { this.getPageNthWord\\(0, 9\\); } catch \\(e\\) { event.target.value = e.name; }",
                "/V ()",
            ),
            "<< >> @stream BT /F1 12 Tf 20 300 Td (one) Tj ET",
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        ],
    );
    let mut view = opened(&document);
    assert_eq!(press(&mut view, &document), "RangeError");
}

#[test]
fn a_group_s_intent_is_table_96_s_with_its_default() {
    let document = document(
        "/AcroForm << /Fields [4 0 R] >> /OCProperties << /OCGs [5 0 R 6 0 R] /D << /ON [5 0 R 6 0 R] >> >>",
        "",
        &[
            &out_field(
                "event.target.value = this.getOCGs\\(\\).map\\(function \\(g\\) { return g.getIntent\\(\\).join\\(\\); }\\).join\\('|'\\);",
                "/V ()",
            ),
            "<< /Type /OCG /Name (Both) /Intent [/View /Design] >>",
            "<< /Type /OCG /Name (Plain) >>",
        ],
    );
    let mut view = opened(&document);
    assert_eq!(press(&mut view, &document), "View,Design|View");
}

#[test]
fn a_rich_text_field_s_keystroke_reads_its_value_as_spans_and_its_change_as_one() {
    // Table 231 bit 26 makes `Out` rich text, and its `/RV` holds the characters its `/V` does,
    // so `event.richValue` is the `/RV` read as spans; the change typed is one span (ADR 1762).
    let rich = "<body xmlns=\"http://www.w3.org/1999/xhtml\"><p>Hello <span \
                style=\"font-weight:bold\">bold</span></p></body>";
    let script = "if \\(!event.willCommit\\) this.getField\\('Log'\\).value = \
                  event.richValue.map\\(function \\(s\\) { return s.text + ':' + s.fontWeight; \
                  }\\).join\\('|'\\) + '#' + event.richChange.length + event.richChange[0].text;";
    let document = document(
        "/AcroForm << /Fields [4 0 R 5 0 R] >>",
        "",
        &[
            &format!(
                "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Out) /Ff 33554432 \
                 /Rect [10 10 210 40] /F 4 /P 3 0 R /DA (/Helv 10 Tf 0 g) /V (Hello bold) \
                 /RV ({rich}) /AA << /K << /S /JavaScript /JS ({script}) >> >> >>"
            ),
            "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Log) /Rect [10 50 210 80] /F 4 \
             /P 3 0 R /DA (/Helv 10 Tf 0 g) /V () >>",
        ],
    );
    let mut view = opened(&document);
    view.set_field(&document, "Out", &Entered::Text("Hello bolder".to_owned()));
    let logged = view
        .field_value(&document, "Log")
        .map(|shown| shown.text)
        .unwrap_or_default();
    assert!(
        logged.starts_with("Hello :400|bold:700") && logged.ends_with("#1Hello bolder"),
        "{logged:?} {:?}",
        view.script_reports()
    );
}

#[test]
fn a_widget_without_a_border_style_reads_its_width_from_border() {
    // Table 166's own example, `[0 0 1 [3 2]]`, is a border one unit wide; with neither entry the
    // border is §12.5.4's one point, and a `/BS` makes `/Border` ignored.
    for (entries, width) in [
        ("/Border [0 0 1 [3 2]]", "1"),
        ("/Border [0 0 4]", "4"),
        ("/Border [0 0 4] /BS << /W 2 >>", "2"),
        ("", "1"),
    ] {
        let document = document(
            "/AcroForm << /Fields [4 0 R] >>",
            "",
            &[&out_field(
                "event.target.value = String\\(event.target.lineWidth\\);",
                &format!("/V () {entries}"),
            )],
        );
        let mut view = opened(&document);
        assert_eq!(press(&mut view, &document), width, "{entries}");
    }
}
