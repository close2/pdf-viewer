//! ADR 1626's members driven through `pdf_model::view::ViewState` with the in-process engine: the
//! commit's key, a full field's two changes, the unsaved mark, the information dictionary, a
//! layer's switch and a push-button's caption — each read from the view state and each change
//! landing in it as an edit (RFC 0008 section 6.4).

#![cfg(feature = "engine")]
#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use std::fmt::Write as _;
use std::sync::Arc;

use pdf_model::view::{CommitKey, Entered, ViewState};
use pdf_script::{Budget, Engine};
use pdf_syntax::{Document, ObjectId};

/// A one-page document whose catalog states `catalog`, whose trailer states `trailer`, and whose
/// objects from 4 are `objects`; the page's annotations are `annots`.
fn document(catalog: &str, trailer: &str, annots: &[u32], objects: &[String]) -> Document {
    let annots: Vec<String> = annots
        .iter()
        .map(|number| format!("{number} 0 R"))
        .collect();
    let mut bodies = vec![
        format!("<< /Type /Catalog /Pages 2 0 R {catalog} >>"),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [{}] >>",
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
        "trailer\n<< /Size {size} /Root 1 0 R {trailer} >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    Document::open(out.into_bytes()).expect("the fixture opens")
}

/// A widget of type `kind` named `name` with `actions` and `extra` entries.
fn widget(kind: &str, name: &str, actions: &str, extra: &str) -> String {
    format!(
        "<< /Type /Annot /Subtype /Widget /FT /{kind} /T ({name}) /Rect [10 10 210 40] /F 4 \
         /P 3 0 R /DA (/Helv 10 Tf 0 g) /AA << {actions} >> {extra} >>"
    )
}

/// A view state of `document` with the engine supplied.
fn engine(document: &Document) -> ViewState {
    let mut view = ViewState::of(document);
    view.run_scripts_with(Some(Arc::new(Engine::new(Budget::FIELD_EVENT))));
    view
}

/// What a field shows, as text.
fn value(view: &ViewState, document: &Document, name: &str) -> String {
    view.field_value(document, name)
        .map(|shown| shown.text)
        .unwrap_or_default()
}

#[test]
fn a_commit_carries_its_key_and_typing_carries_none() {
    let document = document(
        "/AcroForm << /Fields [4 0 R] >>",
        "",
        &[4],
        &[widget(
            "Tx",
            "Amount",
            "/K << /S /JavaScript /JS (event.value = event.willCommit ? 'key ' + event.commitKey \
             : event.value; if \\(!event.willCommit\\) event.change = event.change + event.commitKey;) >>",
            "",
        )],
    );
    let mut view = engine(&document);
    view.set_field(&document, "Amount", &Entered::Text("a".to_owned()));
    assert_eq!(
        value(&view, &document, "Amount"),
        "a0",
        "typing is no commit"
    );
    view.commit_field_by(&document, "Amount", CommitKey::Tab);
    assert_eq!(value(&view, &document, "Amount"), "key 3");
    view.set_field(&document, "Amount", &Entered::Text("b".to_owned()));
    view.commit_field(&document, "Amount");
    assert_eq!(
        value(&view, &document, "Amount"),
        "key 2",
        "a commit unstated is Enter's"
    );
}

#[test]
fn a_full_field_hands_the_script_what_fits_and_what_was_typed() {
    let document = document(
        "/AcroForm << /Fields [4 0 R 5 0 R] >>",
        "",
        &[4, 5],
        &[
            widget(
                "Tx",
                "Code",
                "/K << /S /JavaScript /JS (if \\(event.fieldFull\\) this.getField\\('Seen'\\).value \
                 = event.change + '|' + event.changeEx;) >>",
                "/MaxLen 3",
            ),
            widget("Tx", "Seen", "", ""),
        ],
    );
    let mut view = engine(&document);
    view.set_field(&document, "Code", &Entered::Text("abcdef".to_owned()));
    assert_eq!(
        value(&view, &document, "Seen"),
        "abc|abcdef",
        "{:?}",
        view.script_reports()
    );
    assert_eq!(
        value(&view, &document, "Code"),
        "abc",
        "a full field takes the change cropped to what fits"
    );
}

#[test]
fn dirty_is_the_view_state_s_unsaved_work_since_the_last_save() {
    let document = document(
        "/AcroForm << /Fields [4 0 R 5 0 R] /CO [5 0 R] >>",
        "",
        &[4, 5],
        &[
            widget("Tx", "Amount", "", ""),
            widget(
                "Tx",
                "Mark",
                "/C << /S /JavaScript /JS (event.value = String\\(this.dirty\\);) >>",
                "",
            ),
        ],
    );
    let mut view = engine(&document);
    view.recalculate_with_runner(&document);
    assert_eq!(
        value(&view, &document, "Mark"),
        "false",
        "nothing typed yet"
    );
    view.set_field(&document, "Amount", &Entered::Text("5".to_owned()));
    view.commit_field(&document, "Amount");
    assert_eq!(value(&view, &document, "Mark"), "true");
    view.mark_saved();
    view.recalculate_with_runner(&document);
    assert_eq!(
        value(&view, &document, "Mark"),
        "false",
        "everything was saved"
    );
}

#[test]
fn info_reads_the_trailer_s_information_dictionary() {
    let document = document(
        "/AcroForm << /Fields [4 0 R] >> /OpenAction 5 0 R",
        "/Info 6 0 R",
        &[4],
        &[
            widget("Tx", "Shown", "", ""),
            "<< /S /JavaScript /JS (this.getField\\('Shown'\\).value = this.info.title + ' ' + \
             this.info.CreationDate.getUTCFullYear\\(\\) + ' ' + this.info.Trapped;) >>"
                .to_owned(),
            "<< /Title (A Form) /CreationDate (D:20000612145409Z) /Trapped /False >>".to_owned(),
        ],
    );
    let mut view = engine(&document);
    view.run_open_scripts(&document, 0);
    assert_eq!(
        value(&view, &document, "Shown"),
        "A Form 2000 False",
        "{:?}",
        view.script_reports()
    );
}

#[test]
fn a_layer_is_switched_as_a_person_switches_it_and_a_locked_one_stays() {
    let document = document(
        "/OCProperties << /OCGs [4 0 R 5 0 R] /D << /Order [4 0 R 5 0 R] /Locked [5 0 R] >> >> \
         /OpenAction 6 0 R",
        "",
        &[],
        &[
            "<< /Type /OCG /Name (Watermark) >>".to_owned(),
            "<< /Type /OCG /Name (Locked) >>".to_owned(),
            "<< /S /JavaScript /JS (this.getOCGs\\(\\).forEach\\(function \\(g\\) { g.state = \
             false; }\\);) >>"
                .to_owned(),
        ],
    );
    let mut view = engine(&document);
    view.run_open_scripts(&document, 0);
    let state = |number| {
        view.optional_content().and_then(|content| {
            content.state(ObjectId {
                number,
                generation: 0,
            })
        })
    };
    assert_eq!(state(4), Some(false), "{:?}", view.script_reports());
    assert_eq!(state(5), Some(true), "a locked group stays");
    assert!(
        view.script_reports()
            .iter()
            .any(|sentence| sentence.contains("Locked") && sentence.contains("Table 99")),
        "{:?}",
        view.script_reports()
    );
}

#[test]
fn a_caption_set_by_a_script_is_saved_as_table_192_s_entry() {
    let document = document(
        "/AcroForm << /Fields [4 0 R] >> /OpenAction 5 0 R",
        "",
        &[4],
        &[
            widget("Btn", "Send", "", "/Ff 65536 /MK << /CA (Send) >>"),
            "<< /S /JavaScript /JS (this.getField\\('Send'\\).buttonSetCaption\\('Formular \
             drucken'\\);) >>"
                .to_owned(),
        ],
    );
    let mut view = engine(&document);
    view.run_open_scripts(&document, 0);
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
    let characteristics = saved.get_key(&widget, "MK");
    let caption = characteristics
        .as_dict()
        .map(|mk| saved.get_key(mk, "CA"))
        .and_then(|caption| match caption {
            pdf_syntax::Object::String(bytes) => Some(pdf_syntax::text_string(&bytes)),
            _ => None,
        });
    assert_eq!(
        caption.as_deref(),
        Some("Formular drucken"),
        "{:?}",
        view.script_reports()
    );
}
