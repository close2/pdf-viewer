//! `this.getAnnots`, `this.getAnnot` and the `Annotation` object, over the annotations a realm is
//! told of (ADRs 1700, 1721).
//!
//! Every expected value is one an annotation state this test wrote states, or the order and the
//! refusals ADR 1700 records as choices; the property spellings are Adobe's "Annotation
//! properties", which `pdf_model::view::AnnotationState` names.

#![cfg(feature = "engine")]
#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use pdf_model::aform::Trigger;
use pdf_model::view::{
    AnnotationChange, AnnotationReach, AnnotationState, DocumentState, ScriptEdit, ScriptSite,
};
use pdf_script::{Budget, Ending, Event, Outcome, Realm, Request};

/// One annotation on `page`, numbered `number`.
fn annotation(number: u32, page: u32, kind: &str, author: &str) -> AnnotationState {
    AnnotationState {
        number,
        generation: 0,
        page,
        kind: kind.to_owned(),
        rect: [10.0, 20.0, 110.0, 70.0],
        name: Some(format!("note-{number}")),
        contents: format!("contents of {number}"),
        author: Some(author.to_owned()),
        modified: Some(1_704_465_015_000),
        hidden: false,
        read_only: number == 12,
        // Table 167 as the three filters read it: 10 is not printed, 11 is not seen, and 12 —
        // `ReadOnly` — takes no pointer.
        reach: AnnotationReach {
            printed: number != 10,
            viewed: number != 11,
            interactive: number != 12,
        },
        popup_open: (kind == "Text").then_some(false),
    }
}

/// Three annotations on two pages, in the page order a view state tells them in.
fn annotations() -> Vec<AnnotationState> {
    vec![
        annotation(10, 0, "Text", "Zoe"),
        annotation(11, 0, "FreeText", "Adam"),
        annotation(12, 2, "Square", "Mia"),
    ]
}

/// A request for `script` at a field's calculation, telling the realm of `document`.
fn request(script: &str, document: Option<DocumentState>) -> Request {
    Request {
        site: ScriptSite::Field(Trigger::Calculate),
        field: "Total".to_owned(),
        label: String::new(),
        script: script.to_owned(),
        event: Event {
            value: String::new(),
            change: String::new(),
            selection_start: 0,
            selection_end: 0,
            will_commit: false,
            commit_key: 0,
            field_full: false,
            change_ex: String::new(),
            source: String::new(),
        },
        fields: Vec::new(),
        page: 0,
        pages: 3,
        dirty: false,
        document,
        moment: 1_704_465_015_000,
        utc_offset_seconds: 0,
    }
}

/// A realm told of [`annotations`].
fn realm() -> Realm {
    let mut realm = Realm::new(Budget::FIELD_EVENT).expect("a realm");
    let told = realm.run(&request(
        "",
        Some(DocumentState {
            annotations: annotations(),
            ..DocumentState::default()
        }),
    ));
    assert_eq!(told.ending, Ending::Finished);
    realm
}

/// Runs `script` and answers what it left in `event.value`.
fn value(realm: &mut Realm, script: &str) -> (Outcome, Option<String>) {
    let ran = realm.run(&request(script, None));
    let value = ran.value.clone();
    (ran, value)
}

#[test]
fn get_annots_answers_every_page_in_page_order_or_one_page() {
    let mut realm = realm();
    let (ran, all) = value(
        &mut realm,
        "this.syncAnnotScan(); var a = this.getAnnots(); \
         event.value = a.map(function (x) { return x.name + '@' + x.page; }).join(',');",
    );
    assert_eq!(ran.ending, Ending::Finished, "{ran:?}");
    assert_eq!(all.as_deref(), Some("note-10@0,note-11@0,note-12@2"));
    let (_, one) = value(
        &mut realm,
        "event.value = this.getAnnots({nPage: 2}).length + ' ' + this.getAnnots(0).length;",
    );
    assert_eq!(one.as_deref(), Some("1 2"));
    let (_, none) = value(&mut realm, "event.value = String(this.getAnnots(1));");
    assert_eq!(
        none.as_deref(),
        Some("null"),
        "a page with none answers null"
    );
}

#[test]
fn the_sorts_are_stable_over_page_order_and_reverse_reverses() {
    let mut realm = realm();
    let (_, by_author) = value(
        &mut realm,
        "event.value = this.getAnnots({nSortBy: ANSB_Author, bReverse: true})\
         .map(function (x) { return x.author; }).join(',');",
    );
    assert_eq!(by_author.as_deref(), Some("Zoe,Mia,Adam"));
    let (_, by_type) = value(
        &mut realm,
        "event.value = this.getAnnots({nSortBy: ANSB_Type})\
         .map(function (x) { return x.type; }).join(',');",
    );
    assert_eq!(by_type.as_deref(), Some("FreeText,Square,Text"));
}

#[test]
fn an_annotation_reads_its_ten_properties() {
    let mut realm = realm();
    let (ran, read) = value(
        &mut realm,
        "var a = this.getAnnot(0, 'note-10'); \
         event.value = [a.type, a.page, a.rect.join(' '), a.name, a.contents, a.author, \
         a.modDate.getTime(), a.hidden, a.readOnly, a.popupOpen].join('|');",
    );
    assert_eq!(ran.ending, Ending::Finished, "{ran:?}");
    assert_eq!(
        read.as_deref(),
        Some("Text|0|10 20 110 70|note-10|contents of 10|Zoe|1704465015000|false|false|false")
    );
    let (_, square) = value(
        &mut realm,
        "var a = this.getAnnot(2, 'note-12'); event.value = a.readOnly + ' ' + a.popupOpen;",
    );
    assert_eq!(
        square.as_deref(),
        Some("true undefined"),
        "an annotation with no window has no popupOpen"
    );
    let (_, missing) = value(
        &mut realm,
        "event.value = String(this.getAnnot(1, 'note-10'));",
    );
    assert_eq!(missing.as_deref(), Some("null"));
}

#[test]
fn the_three_writes_a_reader_s_edit_reaches_are_edits_read_back() {
    let mut realm = realm();
    let (ran, read) = value(
        &mut realm,
        "var t = this.getAnnot(0, 'note-10'); t.hidden = true; t.popupOpen = true; \
         var f = this.getAnnot(0, 'note-11'); f.contents = 'retyped'; \
         event.value = t.hidden + ' ' + t.popupOpen + ' ' + f.contents;",
    );
    assert_eq!(ran.ending, Ending::Finished, "{ran:?}");
    assert_eq!(read.as_deref(), Some("true true retyped"));
    assert_eq!(
        ran.edits,
        vec![
            ScriptEdit::Annotation {
                number: 10,
                generation: 0,
                change: AnnotationChange::Hidden(true),
            },
            ScriptEdit::Annotation {
                number: 10,
                generation: 0,
                change: AnnotationChange::PopupOpen(true),
            },
            ScriptEdit::Annotation {
                number: 11,
                generation: 0,
                change: AnnotationChange::Contents("retyped".to_owned()),
            },
        ]
    );
}

#[test]
fn every_other_write_is_refused_by_name() {
    let mut realm = realm();
    for (script, member) in [
        (
            "this.getAnnot(0, 'note-10').author = 'Eve';",
            "Annotation.author=",
        ),
        (
            "this.getAnnot(2, 'note-12').contents = 'x';",
            "Annotation.contents=",
        ),
        (
            "this.getAnnot(2, 'note-12').popupOpen = true;",
            "Annotation.popupOpen=",
        ),
        (
            "this.getAnnots({nFilterBy: ANFB_ShouldAppearInPanel});",
            "this.getAnnots(nFilterBy: ANFB_ShouldAppearInPanel)",
        ),
        (
            "this.getAnnots({nFilterBy: ANFB_ShouldSummarize});",
            "this.getAnnots(nFilterBy: ANFB_ShouldSummarize)",
        ),
        (
            "this.getAnnots({nFilterBy: ANFB_ShouldExport});",
            "this.getAnnots(nFilterBy: ANFB_ShouldExport)",
        ),
        (
            "this.getAnnots({nFilterBy: 7});",
            "this.getAnnots(nFilterBy: a number that names none of the reference's seven)",
        ),
    ] {
        let ran = realm.run(&request(
            &format!("try {{ {script} }} catch (e) {{ console.println(e.name); }}"),
            None,
        ));
        assert_eq!(ran.ending, Ending::Finished, "{script}: {ran:?}");
        assert_eq!(ran.log, vec!["NotAllowedError".to_owned()], "{script}");
        assert_eq!(
            ran.refusals.first().map(|refusal| refusal.member.as_str()),
            Some(member),
            "{script}"
        );
        assert!(ran.edits.is_empty(), "{script}: {:?}", ran.edits);
    }
}

#[test]
fn the_flag_filters_keep_what_table_167_lets_reach_paper_a_screen_and_a_pointer() {
    let mut realm = realm();
    let names = |filter: &str| {
        format!(
            "var a = this.getAnnots({{nFilterBy: {filter}}}); \
             event.value = a === null ? 'null' : a.map(function (x) {{ return x.name; }}).join(',');"
        )
    };
    for (filter, kept) in [
        ("ANFB_ShouldNone", "note-10,note-11,note-12"),
        ("ANFB_ShouldPrint", "note-11,note-12"),
        ("ANFB_ShouldView", "note-10,note-12"),
        ("ANFB_ShouldEdit", "note-10,note-11"),
    ] {
        let (ran, read) = value(&mut realm, &names(filter));
        assert_eq!(ran.ending, Ending::Finished, "{filter}: {ran:?}");
        assert_eq!(read.as_deref(), Some(kept), "{filter}");
    }
    // Table 167's `Hidden` suppresses an annotation on paper, on a screen and for a pointer, and
    // the filter reads the bit a script has just written.
    let (_, after) = value(
        &mut realm,
        &format!(
            "this.getAnnot(2, 'note-12').hidden = true; {}",
            names("ANFB_ShouldPrint")
        ),
    );
    assert_eq!(after.as_deref(), Some("note-11"));
}

#[test]
fn a_text_note_s_contents_is_written_as_a_free_text_annotation_s_is() {
    let mut realm = realm();
    let (ran, read) = value(
        &mut realm,
        "var t = this.getAnnot(0, 'note-10'); t.contents = 'new note'; event.value = t.contents;",
    );
    assert_eq!(ran.ending, Ending::Finished, "{ran:?}");
    assert_eq!(read.as_deref(), Some("new note"));
    assert_eq!(
        ran.edits,
        vec![ScriptEdit::Annotation {
            number: 10,
            generation: 0,
            change: AnnotationChange::Contents("new note".to_owned()),
        }]
    );
}

#[test]
fn a_stopped_run_leaves_the_annotations_as_they_were() {
    let mut realm = realm();
    let ran = realm.run(&request(
        "this.getAnnot(0, 'note-10').hidden = true; while (true) {}",
        None,
    ));
    assert!(matches!(ran.ending, Ending::Exceeded(_)), "{ran:?}");
    assert!(ran.edits.is_empty());
    let (_, after) = value(
        &mut realm,
        "event.value = String(this.getAnnot(0, 'note-10').hidden);",
    );
    assert_eq!(after.as_deref(), Some("false"));
}

#[test]
fn a_document_with_no_annotation_answers_null() {
    let mut realm = Realm::new(Budget::FIELD_EVENT).expect("a realm");
    let ran = realm.run(&request(
        "event.value = String(this.getAnnots());",
        Some(DocumentState::default()),
    ));
    assert_eq!(ran.value.as_deref(), Some("null"), "{ran:?}");
}

/// A document whose open action is `script`, its one page carrying a text annotation (4) with its
/// popup (5) and a free text annotation (6).
fn opened(script: &str) -> pdf_syntax::Document {
    use std::fmt::Write as _;
    let objects = [
        format!(
            "<< /Type /Catalog /Pages 2 0 R /OpenAction << /S /JavaScript /JS ({script}) >> >>"
        ),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [4 0 R 5 0 R 6 0 R] >>"
            .to_owned(),
        "<< /Type /Annot /Subtype /Text /Rect [100 300 120 320] /NM (note) /F 4 /Popup 5 0 R >>"
            .to_owned(),
        "<< /Type /Annot /Subtype /Popup /Rect [150 200 350 300] /Parent 4 0 R >>".to_owned(),
        "<< /Type /Annot /Subtype /FreeText /Rect [10 10 210 60] /Contents (Old) \
         /DA (/Helv 12 Tf 0 g) >>"
            .to_owned(),
    ];
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
    pdf_syntax::Document::open(out.into_bytes()).expect("the fixture opens")
}

#[test]
fn a_script_s_writes_reach_the_saved_file_through_the_view_state() {
    let document = opened(
        "var a = this.getAnnots\\(\\); a[0].hidden = true; a[0].popupOpen = true; \
         a[1].contents = 'New';",
    );
    let mut view = pdf_model::view::ViewState::of(&document);
    view.run_scripts_with(Some(std::sync::Arc::new(pdf_script::Engine::new(
        Budget::FIELD_EVENT,
    ))));
    view.run_open_scripts(&document, 0);
    assert!(
        view.script_reports().is_empty(),
        "{:?}",
        view.script_reports()
    );
    let written = view.save(&document).expect("the update writes");
    let saved = pdf_syntax::Document::open(written.bytes).expect("the update reads back");
    let object = |number| {
        saved
            .get(pdf_syntax::ObjectId {
                number,
                generation: 0,
            })
            .as_dict()
            .cloned()
            .expect("a dictionary")
    };
    // Table 167: bit 2, Hidden, beside the file's bit 3.
    assert_eq!(
        saved.get_key(&object(4), "F"),
        pdf_syntax::Object::Integer(6)
    );
    // Table 186's `/Open`.
    assert_eq!(
        saved.get_key(&object(5), "Open"),
        pdf_syntax::Object::Boolean(true)
    );
    // Table 166's `/Contents`, as a person's retyping writes it.
    assert_eq!(
        saved.get_key(&object(6), "Contents"),
        pdf_syntax::Object::String(b"New".to_vec().into())
    );
}
