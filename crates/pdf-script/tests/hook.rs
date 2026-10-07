//! RFC 0008 section 6.3's hook, driven through `pdf_model::view::ViewState`: a script Tier 0 does
//! not run goes to a supplied runner, and with none supplied — the level `off`, the default —
//! nothing runs (ADR 1591); with the engine supplied, the open sequence, Table 197's and Table 198's
//! sites and a commit's `/V`, `/C` over `/CO` and `/F` run in the order the standard and the
//! reference give, and every change a script makes is an edit in the view state (ADRs 1602, 1603).

#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

use pdf_model::aform::Trigger;
use pdf_model::view::{Entered, ScriptEvent, ScriptResult, ScriptRunner, ScriptSite, ViewState};
use pdf_syntax::Document;

/// A one-page document: the catalog states `catalog` beside `/Pages`, the page states `page` and
/// annotations `annots`, and `objects` are numbered from 4.
fn document(catalog: &str, page: &str, annots: &[u32], objects: &[String]) -> Document {
    let annots: Vec<String> = annots
        .iter()
        .map(|number| format!("{number} 0 R"))
        .collect();
    let mut bodies = vec![
        format!("<< /Type /Catalog /Pages 2 0 R {catalog} >>"),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [{}] {page} >>",
            annots.join(" ")
        ),
    ];
    bodies.extend(objects.iter().cloned());
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

/// A text field widget named `name`, its `/AA` holding `actions`, and `extra` entries.
fn text_field(name: &str, actions: &str, extra: &str) -> String {
    format!(
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T ({name}) /Rect [10 10 210 40] /F 4 /P 3 0 R \
         /DA (/Helv 10 Tf 0 g) /AA << {actions} >> {extra} >>"
    )
}

/// A one-field document with `Amount`, whose `/AA` holds `actions`.
fn one_field(actions: &str) -> Document {
    document(
        "/AcroForm << /Fields [4 0 R] >>",
        "",
        &[4],
        &[text_field("Amount", actions, "")],
    )
}

/// A runner that records what it was handed and answers as told.
#[derive(Debug, Default)]
struct Recording {
    /// Each event's site, script, value and change.
    seen: Mutex<Vec<(ScriptSite, String, String, String)>>,
}

impl ScriptRunner for Recording {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        if let Ok(mut seen) = self.seen.lock() {
            seen.push((
                event.site,
                event.script.to_owned(),
                event.value.to_owned(),
                event.change.to_owned(),
            ));
        }
        ScriptResult {
            rc: event.change != "x",
            value: (event.site == ScriptSite::Field(Trigger::Format))
                .then(|| format!("<{}>", event.value)),
            change: None,
            edits: Vec::new(),
            report: vec!["the runner ran".to_owned()],
        }
    }
}

const SCRIPTS: &str = "/K << /S /JavaScript /JS (if \\(event.change == 'x'\\) event.rc = false;) >> \
                       /F << /S /JavaScript /JS (event.value = '<' + event.value + '>';) >>";

#[test]
fn with_no_runner_supplied_nothing_runs_and_each_script_is_reported() {
    let document = one_field(SCRIPTS);
    let mut view = ViewState::of(&document);
    assert!(view.set_field(&document, "Amount", &Entered::Text("x".to_owned())) > 0);
    assert_eq!(
        view.displayed_value(&document, "Amount").as_deref(),
        Some("x")
    );
    assert!(
        view.script_reports()
            .iter()
            .all(|sentence| sentence.contains("a script this tier does not run")),
        "{:?}",
        view.script_reports()
    );
}

#[test]
fn a_supplied_runner_is_handed_the_keystroke_and_the_format() {
    let document = one_field(SCRIPTS);
    let runner = Arc::new(Recording::default());
    let mut view = ViewState::of(&document);
    view.run_scripts_with(Some(runner.clone()));
    assert_eq!(
        view.set_field(&document, "Amount", &Entered::Text("x".to_owned())),
        0,
        "the runner's rc false rejects the keystroke"
    );
    assert!(view.set_field(&document, "Amount", &Entered::Text("12".to_owned())) > 0);
    view.commit_field(&document, "Amount");
    assert_eq!(
        view.displayed_value(&document, "Amount").as_deref(),
        Some("<12>")
    );
    let seen = runner.seen.lock().expect("the record").clone();
    let sites: Vec<ScriptSite> = seen.iter().map(|(site, ..)| *site).collect();
    assert_eq!(
        sites,
        vec![
            ScriptSite::Field(Trigger::Keystroke),
            ScriptSite::Field(Trigger::Keystroke),
            ScriptSite::Field(Trigger::Keystroke),
            ScriptSite::Field(Trigger::Format)
        ],
        "the commit's format runs once and is kept"
    );
    assert!(seen.iter().all(|(_, script, ..)| !script.is_empty()));
    assert!(
        view.script_reports()
            .iter()
            .any(|sentence| sentence == "Amount: the runner ran"),
        "{:?}",
        view.script_reports()
    );
    view.run_scripts_with(None);
    assert_eq!(
        view.displayed_value(&document, "Amount").as_deref(),
        Some("12"),
        "taking the runner back is the level off again"
    );
}

#[test]
fn with_no_runner_the_open_says_how_many_document_level_scripts_went_unrun() {
    let document = document(
        "/AcroForm << /Fields [4 0 R] >> /Names << /JavaScript << /Names [(a) 5 0 R (b) 6 0 R] >> >>",
        "",
        &[4],
        &[
            text_field("Amount", "", ""),
            "<< /S /JavaScript /JS (var a = 1;) >>".to_owned(),
            "<< /S /JavaScript /JS (var b = 2;) >>".to_owned(),
        ],
    );
    let mut view = ViewState::of(&document);
    assert_eq!(view.run_open_scripts(&document, 0), 0);
    assert!(
        view.script_reports()
            .iter()
            .any(|sentence| sentence.contains("2 document-level script(s)")),
        "{:?}",
        view.script_reports()
    );
}

#[cfg(feature = "engine")]
mod engine {
    use super::{Entered, ViewState, document, one_field, text_field};
    use pdf_model::action::{PageTrigger, Trigger as AnnotationTrigger};
    use pdf_model::view::{Committed, Property};
    use pdf_script::{Budget, Engine};
    use pdf_syntax::{Document, ObjectId};

    /// A view state with the engine supplied.
    fn engine_view(document: &Document) -> ViewState {
        let mut view = ViewState::of(document);
        view.run_scripts_with(Some(Engine::runner(Budget::FIELD_EVENT)));
        view
    }

    /// The text a field holds now.
    fn value(view: &ViewState, document: &Document, name: &str) -> String {
        view.field_value(document, name)
            .map(|shown| shown.text)
            .unwrap_or_default()
    }

    /// The object identity of object `number`.
    fn id(number: u32) -> ObjectId {
        ObjectId {
            number,
            generation: 0,
        }
    }

    #[test]
    fn the_engine_rejects_formats_and_refuses_a_commit_through_the_view_state() {
        let document = one_field(
            "/K << /S /JavaScript /JS (if \\(!event.willCommit && !/^[0-9.]*$/.test\\(event.change\\)\\) \
             event.rc = false; if \\(event.willCommit && event.value == '13'\\) event.rc = false;) >> \
             /F << /S /JavaScript /JS (if \\(event.value != ''\\) AFNumber_Format\\(2, 0, 0, 0, '$', true\\);) >>",
        );
        let mut view = engine_view(&document);
        assert_eq!(
            view.set_field(&document, "Amount", &Entered::Text("1a".to_owned())),
            0
        );
        assert!(view.set_field(&document, "Amount", &Entered::Text("1234.5".to_owned())) > 0);
        assert_eq!(view.commit_field(&document, "Amount"), Committed::Accepted);
        assert_eq!(
            view.displayed_value(&document, "Amount").as_deref(),
            Some("$1,234.50")
        );
        assert!(view.set_field(&document, "Amount", &Entered::Text("13".to_owned())) > 0);
        assert!(matches!(
            view.commit_field(&document, "Amount"),
            Committed::Refused(_)
        ));
        assert_eq!(
            value(&view, &document, "Amount"),
            "1234.5",
            "a refused commit puts back what was there"
        );
    }

    #[test]
    fn a_runaway_format_leaves_the_value_as_it_stands() {
        let document = one_field("/F << /S /JavaScript /JS (while \\(true\\) {}) >>");
        let mut view = engine_view(&document);
        assert!(view.set_field(&document, "Amount", &Entered::Text("7".to_owned())) > 0);
        view.commit_field(&document, "Amount");
        assert_eq!(
            view.displayed_value(&document, "Amount").as_deref(),
            Some("7")
        );
    }

    #[test]
    fn the_open_runs_the_name_tree_the_open_action_the_page_and_its_annotations_in_order() {
        // §12.6.4.17: the name tree's scripts run "[w]hen the document is opened", defining
        // functions other scripts call; Table 198's /O "shall be executed after" the catalog's
        // /OpenAction; Table 197's /PO "shall be executed after the O action". So `mark`,
        // defined by the tree's first entry in key order, sees L, then A, then O, then P.
        let document = document(
            "/AcroForm << /Fields [4 0 R] >> \
             /Names << /JavaScript << /Names [(1 define) 5 0 R (2 mark) 6 0 R] >> >> \
             /OpenAction << /S /JavaScript /JS (mark\\('A'\\);) >>",
            "/AA << /O << /S /JavaScript /JS (mark\\('O'\\);) >> >>",
            &[4],
            &[
                text_field(
                    "Log",
                    "/PO << /S /JavaScript /JS (mark\\('P'\\);) >>",
                    "/V ()",
                ),
                "<< /S /JavaScript /JS (function mark\\(s\\) { var f = this.getField\\('Log'\\); \
                 f.value = f.valueAsString + s; }) >>"
                    .to_owned(),
                "<< /S /JavaScript /JS (mark\\('L'\\);) >>".to_owned(),
            ],
        );
        let mut view = engine_view(&document);
        assert_eq!(view.run_open_scripts(&document, 0), 5);
        assert_eq!(
            value(&view, &document, "Log"),
            "LAOP",
            "{:?}",
            view.script_reports()
        );
        assert!(
            view.script_reports().is_empty(),
            "{:?}",
            view.script_reports()
        );
    }

    #[test]
    fn a_page_closing_runs_its_annotations_first_and_then_its_own_script() {
        // Table 197's /PC "shall be executed before the C action".
        let document = document(
            "/AcroForm << /Fields [4 0 R] >>",
            "/AA << /C << /S /JavaScript /JS (this.getField\\('Log'\\).value += 'C';) >> >>",
            &[4],
            &[text_field(
                "Log",
                "/PC << /S /JavaScript /JS (this.getField\\('Log'\\).value += 'P';) >>",
                "/V ()",
            )],
        );
        let mut view = engine_view(&document);
        assert_eq!(view.run_page_scripts(&document, 0, PageTrigger::Close), 2);
        assert_eq!(value(&view, &document, "Log"), "PC");
    }

    #[test]
    fn a_button_s_mouse_up_script_writes_a_field_and_resets_another() {
        // Table 197's /U, with its /A taking precedence; the script's writes are edits, and a
        // reset is §12.7.6.3's, taking the field back to its /DV.
        let document = document(
            "/AcroForm << /Fields [4 0 R 5 0 R 6 0 R] >>",
            "",
            &[4, 5, 6],
            &[
                "<< /Type /Annot /Subtype /Widget /FT /Btn /Ff 65536 /T (Go) /Rect [0 0 20 20] \
                 /A << /S /JavaScript /JS (this.getField\\('Out'\\).value = 'pressed'; \
                 this.resetForm\\(['Kept']\\);) >> \
                 /AA << /U << /S /JavaScript /JS (this.getField\\('Out'\\).value = 'not reached';) >> >> >>"
                    .to_owned(),
                text_field("Out", "", "/V (before)"),
                text_field("Kept", "", "/V (typed) /DV (default)"),
            ],
        );
        let mut view = engine_view(&document);
        assert_eq!(
            view.run_annotation_scripts(&document, id(4), AnnotationTrigger::Up),
            1
        );
        assert_eq!(value(&view, &document, "Out"), "pressed");
        assert_eq!(value(&view, &document, "Kept"), "default");
    }

    /// A form whose `/CO` is `Total`, then `Runaway`, then `Double`, over `A` and `B`.
    fn calculated(runaway: &str) -> Document {
        document(
            "/AcroForm << /Fields [4 0 R 5 0 R 6 0 R 7 0 R 8 0 R] /CO [6 0 R 7 0 R 8 0 R] >>",
            "",
            &[4, 5, 6, 7, 8],
            &[
                text_field("A", "", "/V (1)"),
                text_field("B", "", "/V (2)"),
                text_field(
                    "Total",
                    "/C << /S /JavaScript /JS (event.value = this.getField\\('A'\\).value + \
                     this.getField\\('B'\\).value + ' from ' + event.source.name;) >>",
                    "",
                ),
                text_field(
                    "Runaway",
                    &format!("/C << /S /JavaScript /JS ({runaway}) >>"),
                    "",
                ),
                text_field(
                    "Double",
                    "/C << /S /JavaScript /JS (AFSimple_Calculate\\('SUM', ['A', 'B']\\); \
                     event.value = 2 * event.value;) >>",
                    "",
                ),
            ],
        )
    }

    #[test]
    fn a_commit_walks_the_calculation_order_through_every_script() {
        let document = calculated("event.value = 'ran';");
        let mut view = engine_view(&document);
        assert!(view.set_field(&document, "A", &Entered::Text("5".to_owned())) > 0);
        assert_eq!(
            value(&view, &document, "Total"),
            "",
            "a script's calculation waits for the commit"
        );
        assert_eq!(view.commit_field(&document, "A"), Committed::Accepted);
        assert_eq!(value(&view, &document, "Total"), "7 from A");
        assert_eq!(value(&view, &document, "Runaway"), "ran");
        assert_eq!(value(&view, &document, "Double"), "14");
    }

    #[test]
    fn a_runaway_calculation_is_stopped_with_its_place_in_the_order_named() {
        let document = calculated("while \\(true\\) {}");
        let mut view = engine_view(&document);
        assert!(view.set_field(&document, "B", &Entered::Text("3".to_owned())) > 0);
        assert_eq!(view.commit_field(&document, "B"), Committed::Accepted);
        let reports = view.script_reports();
        assert!(
            reports.iter().any(|sentence| sentence.starts_with(
                "Runaway (entry 2 of 3 in the calculation order): the script was stopped"
            )),
            "{reports:?}"
        );
        assert_eq!(
            value(&view, &document, "Runaway"),
            "",
            "a stopped run changes nothing"
        );
        assert_eq!(
            value(&view, &document, "Double"),
            "8",
            "the walk goes on past a stopped entry"
        );
    }

    #[test]
    fn a_validate_script_s_rc_false_leaves_the_field_unchanged_and_says_so() {
        let document =
            one_field("/V << /S /JavaScript /JS (if \\(event.value > 100\\) event.rc = false;) >>");
        let mut view = engine_view(&document);
        assert!(view.set_field(&document, "Amount", &Entered::Text("50".to_owned())) > 0);
        assert_eq!(view.commit_field(&document, "Amount"), Committed::Accepted);
        assert!(view.set_field(&document, "Amount", &Entered::Text("500".to_owned())) > 0);
        match view.commit_field(&document, "Amount") {
            Committed::Refused(sentence) => assert!(
                sentence.contains("validate script refused the value"),
                "{sentence}"
            ),
            other => panic!("a validate script's rc false refuses, not {other:?}"),
        }
        assert_eq!(value(&view, &document, "Amount"), "50");
    }

    #[test]
    fn the_drawn_appearance_is_the_runner_s_format() {
        let document =
            one_field("/F << /S /JavaScript /JS (event.value = '<' + event.value + '>';) >>");
        let mut view = engine_view(&document);
        assert!(view.set_field(&document, "Amount", &Entered::Text("12".to_owned())) > 0);
        assert_eq!(view.commit_field(&document, "Amount"), Committed::Accepted);
        let displayed = view.annotation(id(4)).displayed.cloned();
        assert_eq!(displayed.map(|shown| shown.shown).as_deref(), Some("<12>"));
        let bytes = view.save(&document).expect("the update writes").bytes;
        let saved = Document::open(bytes).expect("the update reads back");
        let object = saved.get(id(4));
        let widget = object.as_dict().expect("a widget");
        let normal = saved
            .get_key(widget, "AP")
            .as_dict()
            .map(|appearances| saved.get_key(appearances, "N"))
            .expect("an /AP");
        let stream = normal.as_stream().expect("a stream");
        let content = String::from_utf8_lossy(&saved.decoded_stream_data(stream).expect("decodes"))
            .into_owned();
        assert!(content.contains("(<12>) Tj"), "{content}");
    }

    #[test]
    fn a_script_s_display_and_readonly_are_edits_a_person_meets() {
        let document = document(
            "/AcroForm << /Fields [4 0 R 5 0 R] >>",
            "",
            &[4, 5],
            &[
                text_field(
                    "Trigger",
                    "/Bl << /S /JavaScript /JS (var f = this.getField\\('Target'\\); \
                     f.display = display.hidden; f.readonly = true; f.textColor = color.red;) >>",
                    "",
                ),
                text_field("Target", "", "/V (kept)"),
            ],
        );
        let mut view = engine_view(&document);
        assert_eq!(
            view.run_annotation_scripts(&document, id(4), AnnotationTrigger::Blur),
            1
        );
        assert_eq!(view.annotation_hidden(id(5)), Some(true));
        assert_eq!(
            view.set_field(&document, "Target", &Entered::Text("typed".to_owned())),
            0,
            "a script's readonly bars the person"
        );
        assert_eq!(value(&view, &document, "Target"), "kept");
        assert!(
            view.script_properties("Target")
                .iter()
                .any(|property| matches!(property, Property::TextColor(_)))
        );
        assert!(
            view.script_reports()
                .iter()
                .any(|sentence| sentence.contains(
                    "Target: a script set Field.textColor; this view state keeps it and the drawn \
                 appearance does not carry it"
                )),
            "{:?}",
            view.script_reports()
        );
    }
}
