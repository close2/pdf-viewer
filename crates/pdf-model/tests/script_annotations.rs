//! What a document's realm is told of its annotations, and how a script's change to one is made and
//! saved (ADRs 1700, 1720, 1721).
//!
//! The fixture's page carries a text annotation with its popup, a free text annotation, a link and
//! a widget. A runner of this test's own records what the view state told it and answers the
//! `/OpenAction` with the three changes a script's `Annotation` object makes; the witnesses are
//! the realm's record and the saved file — Table 167's `Hidden` bit in `/F`, Table 186's `/Open`
//! and Table 166's `/Contents`, each where the standard keeps it.

#![expect(
    clippy::expect_used,
    reason = "test code: an explanatory panic is the intended failure"
)]

use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

use pdf_model::view::{
    AnnotationChange, AnnotationState, ScriptEdit, ScriptEvent, ScriptResult, ScriptRunner,
    ScriptSite, ViewState,
};
use pdf_syntax::{Dictionary, Document, Object, ObjectId};

/// A document of `objects`, numbered from 1, the first the catalog.
fn assembled(objects: &[&str]) -> Document {
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

/// Two pages; the first, whose Table 198 `/O` is a script, carries a text annotation (4) and its
/// popup (5), a free text annotation (6), a link (7) and a widget (8), the second a square (10).
fn document() -> Document {
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R /OpenAction << /S /JavaScript /JS (change\\(\\);) >> \
         /AcroForm << /Fields [8 0 R] /DR << /Font << /Helv 11 0 R >> >> >> >>",
        "<< /Type /Pages /Kids [3 0 R 9 0 R] /Count 2 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] \
         /Annots [4 0 R 5 0 R 6 0 R 7 0 R 8 0 R] /AA << /O << /S /JavaScript /JS (read\\(\\);) >> >> >>",
        "<< /Type /Annot /Subtype /Text /Rect [120 300 100 320] /NM (note) /T (Ann) \
         /Contents (Read me) /M (D:20240105143015Z) /F 4 /Popup 5 0 R >>",
        "<< /Type /Annot /Subtype /Popup /Rect [150 200 350 300] /Parent 4 0 R >>",
        "<< /Type /Annot /Subtype /FreeText /Rect [10 10 210 60] /Contents (Old) \
         /DA (/Helv 12 Tf 0 g) /F 68 >>",
        "<< /Type /Annot /Subtype /Link /Rect [0 0 10 10] >>",
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Field) /Rect [10 100 210 130] /P 3 0 R >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [10 0 R] >>",
        "<< /Type /Annot /Subtype /Square /Rect [0 0 50 50] /NM (box) >>",
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    ])
}

/// A runner that records each telling of the annotations and answers the open action with the
/// three changes.
#[derive(Debug, Default)]
struct Changing {
    /// Every annotation list the view state told, in order.
    told: Mutex<Vec<Vec<AnnotationState>>>,
}

impl ScriptRunner for Changing {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        if let (Some(document), Ok(mut told)) = (event.document, self.told.lock()) {
            told.push(document.annotations.clone());
        }
        let change = |number, change| ScriptEdit::Annotation {
            number,
            generation: 0,
            change,
        };
        let edits = if event.site == ScriptSite::OpenAction {
            vec![
                change(4, AnnotationChange::Hidden(true)),
                change(4, AnnotationChange::PopupOpen(true)),
                change(6, AnnotationChange::Contents("Retyped".to_owned())),
            ]
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

#[test]
#[expect(
    clippy::float_cmp,
    reason = "the fixture writes whole numbers, which a double holds exactly"
)]
fn a_realm_is_told_every_page_s_scripted_annotations_in_page_order() {
    let document = document();
    let runner = Arc::new(Changing::default());
    let mut view = ViewState::of(&document);
    view.run_scripts_with(Some(Arc::clone(&runner) as Arc<dyn ScriptRunner>));
    view.run_open_scripts(&document, 0);
    let told = runner.told.lock().expect("the record").clone();
    let first = told.first().expect("the realm is told of the document");
    // Table 166's `/Subtype`: the popup, the link and the widget are not a script's annotations.
    assert_eq!(
        first
            .iter()
            .map(|annotation| (annotation.number, annotation.page, annotation.kind.as_str()))
            .collect::<Vec<_>>(),
        vec![(4, 0, "Text"), (6, 0, "FreeText"), (10, 1, "Square")]
    );
    let note = &first[0];
    // §7.9.5: a rectangle's corners are read lower left first, whatever order the file wrote.
    assert_eq!(note.rect, [100.0, 300.0, 120.0, 320.0]);
    assert_eq!(note.name.as_deref(), Some("note"));
    assert_eq!(note.author.as_deref(), Some("Ann"));
    assert_eq!(note.contents, "Read me");
    // §7.9.4: 2024-01-05T14:30:15Z.
    assert_eq!(note.modified, Some(1_704_465_015_000));
    assert!(!note.hidden && !note.read_only);
    assert_eq!(
        note.popup_open,
        Some(false),
        "Table 186's /Open defaults to false"
    );
    let free_text = &first[1];
    // Table 167: 68 is bits 3 and 7, Print and ReadOnly.
    assert!(free_text.read_only && !free_text.hidden);
    assert_eq!(
        free_text.popup_open, None,
        "a free text annotation has no popupOpen"
    );
}

#[test]
fn a_script_s_three_changes_are_drawn_retold_and_saved() {
    let document = document();
    let runner = Arc::new(Changing::default());
    let mut view = ViewState::of(&document);
    view.run_scripts_with(Some(Arc::clone(&runner) as Arc<dyn ScriptRunner>));
    view.run_open_scripts(&document, 0);
    let note = ObjectId {
        number: 4,
        generation: 0,
    };
    assert_eq!(view.annotation_hidden(note), Some(true));
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let windows = pdf_model::popup::popups(&document, &page, &view);
    assert!(
        windows
            .iter()
            .any(|window| window.annotation.number == 5 && window.open),
        "the window opens: {windows:?}"
    );
    // Page one's `/O` runs after the open action, and its telling carries what the changes made.
    let told = runner.told.lock().expect("the record").clone();
    let last = told.last().expect("a telling");
    assert!(last[0].hidden && last[0].popup_open == Some(true));
    assert_eq!(last[1].contents, "Retyped");

    let written = view.save(&document).expect("the update writes");
    let saved = Document::open(written.bytes).expect("the update reads back");
    // Table 167 bit 2 joins bit 3, which the file stated.
    assert_eq!(saved.get_key(&object(&saved, 4), "F"), Object::Integer(6));
    assert_eq!(
        saved.get_key(&object(&saved, 5), "Open"),
        Object::Boolean(true)
    );
    assert_eq!(
        saved.get_key(&object(&saved, 6), "Contents"),
        Object::String(b"Retyped".to_vec().into())
    );
}

/// A runner that records the page each annotation event says it is on, and closes the square's
/// page's text note.
#[derive(Debug, Default)]
struct Paging {
    /// Each annotation event's `this.pageNum`.
    pages: Mutex<Vec<usize>>,
}

impl ScriptRunner for Paging {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        let mut edits = Vec::new();
        if matches!(event.site, ScriptSite::Annotation(_)) {
            if let Ok(mut pages) = self.pages.lock() {
                pages.push(event.page);
            }
            edits.push(ScriptEdit::Annotation {
                number: 5,
                generation: 0,
                change: AnnotationChange::PopupOpen(false),
            });
        }
        ScriptResult {
            rc: true,
            value: None,
            change: None,
            edits,
            report: Vec::new(),
        }
    }
}

#[test]
fn an_annotation_with_no_page_entry_is_on_the_page_that_lists_it() {
    // The square (4) states no Table 166 `/P`, and is page two's; the text note (5) has no popup,
    // so Table 175's `/Open` is its window's whole statement.
    let document = assembled(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R 6 0 R] /Count 2 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] >>",
        "<< /Type /Annot /Subtype /Square /Rect [0 0 50 50] \
         /AA << /U << /S /JavaScript /JS (close\\(\\);) >> >> >>",
        "<< /Type /Annot /Subtype /Text /Rect [60 60 80 80] /Open true >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [4 0 R 5 0 R] >>",
    ]);
    let runner = Arc::new(Paging::default());
    let mut view = ViewState::of(&document);
    view.run_scripts_with(Some(Arc::clone(&runner) as Arc<dyn ScriptRunner>));
    // A widget's event is a host's after the open sequence (ADR 1750); page one has no script.
    view.run_open_scripts(&document, 0);
    let square = ObjectId {
        number: 4,
        generation: 0,
    };
    let ran = view.run_annotation_scripts(&document, square, pdf_model::action::Trigger::Up);
    assert_eq!(ran.handed, 1);
    assert!(
        ran.changed,
        "the script opened a popup, which the page draws"
    );
    assert_eq!(*runner.pages.lock().expect("the record"), vec![1]);
    let written = view.save(&document).expect("the update writes");
    let saved = Document::open(written.bytes).expect("the update reads back");
    assert_eq!(
        saved.get_key(&object(&saved, 5), "Open"),
        Object::Boolean(false)
    );
}

/// A runner that answers each run with the edits the test queued for it, and records each
/// telling's annotations.
#[derive(Debug, Default)]
struct Queued {
    /// What the next run answers with.
    next: Mutex<Vec<ScriptEdit>>,
    /// Every annotation list the view state told, in order.
    told: Mutex<Vec<Vec<AnnotationState>>>,
}

impl Queued {
    /// Queues one change to annotation `number` for the next run.
    fn queue(&self, number: u32, change: AnnotationChange) {
        self.next
            .lock()
            .expect("the queue")
            .push(ScriptEdit::Annotation {
                number,
                generation: 0,
                change,
            });
    }
}

impl ScriptRunner for Queued {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        if let (Some(document), Ok(mut told)) = (event.document, self.told.lock()) {
            told.push(document.annotations.clone());
        }
        let edits = self
            .next
            .lock()
            .map(|mut next| std::mem::take(&mut *next))
            .unwrap_or_default();
        ScriptResult {
            rc: true,
            value: None,
            change: None,
            edits,
            report: Vec::new(),
        }
    }
}

/// One page: a text note (4) with its popup (5), and a square (6) whose Table 197 `/U` is a script;
/// the catalog's `/OpenAction` is one too.
fn windowed() -> Document {
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R /OpenAction << /S /JavaScript /JS (open\\(\\);) >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [4 0 R 5 0 R 6 0 R] >>",
        "<< /Type /Annot /Subtype /Text /Rect [100 300 120 320] /Contents (Read me) \
         /Popup 5 0 R >>",
        "<< /Type /Annot /Subtype /Popup /Rect [150 200 350 300] /Parent 4 0 R >>",
        "<< /Type /Annot /Subtype /Square /Rect [0 0 50 50] \
         /AA << /U << /S /JavaScript /JS (later\\(\\);) >> >> >>",
    ])
}

/// Whether popup 5's window is open on page one, as `crate::popup` draws it.
fn window_open(document: &Document, view: &ViewState) -> bool {
    let page = pdf_model::Pages::new(document).get(0).expect("page one");
    pdf_model::popup::popups(document, &page, view)
        .iter()
        .any(|window| window.annotation.number == 5 && window.open)
}

/// Popup 5's `/Open` in what `view` saves.
fn saved_open(document: &Document, view: &ViewState) -> Object {
    let written = view.save(document).expect("the update writes");
    let saved = Document::open(written.bytes).expect("the update reads back");
    saved.get_key(&object(&saved, 5), "Open")
}

const POPUP: ObjectId = ObjectId {
    number: 5,
    generation: 0,
};
const SQUARE: ObjectId = ObjectId {
    number: 6,
    generation: 0,
};

#[test]
fn a_person_s_click_after_a_script_s_popup_open_wins() {
    let document = windowed();
    let runner = Arc::new(Queued::default());
    let mut view = ViewState::of(&document);
    view.run_scripts_with(Some(Arc::clone(&runner) as Arc<dyn ScriptRunner>));
    runner.queue(4, AnnotationChange::PopupOpen(true));
    view.run_open_scripts(&document, 0);
    assert!(window_open(&document, &view), "the script opened it");
    // What `viewer-core`'s click on the note does (ADR 1720).
    view.set_popup_open(POPUP, false);
    assert!(!window_open(&document, &view), "the later click closed it");
    // The realm reads the window as it is shown.
    view.run_annotation_scripts(&document, SQUARE, pdf_model::action::Trigger::Up);
    let told = runner.told.lock().expect("the record").clone();
    assert_eq!(told.last().expect("a telling")[0].popup_open, Some(false));
    // The script's write is saved as the window is shown when the save is made.
    assert_eq!(saved_open(&document, &view), Object::Boolean(false));
}

#[test]
fn a_script_s_popup_open_after_a_person_s_click_wins() {
    let document = windowed();
    let runner = Arc::new(Queued::default());
    let mut view = ViewState::of(&document);
    view.run_scripts_with(Some(Arc::clone(&runner) as Arc<dyn ScriptRunner>));
    view.run_open_scripts(&document, 0);
    view.set_popup_open(POPUP, true);
    assert!(window_open(&document, &view), "the click opened it");
    runner.queue(4, AnnotationChange::PopupOpen(false));
    view.run_annotation_scripts(&document, SQUARE, pdf_model::action::Trigger::Up);
    assert!(!window_open(&document, &view), "the later script closed it");
    assert_eq!(saved_open(&document, &view), Object::Boolean(false));
}

#[test]
fn a_person_s_click_alone_is_not_saved() {
    // Table 186's `/Open` is how the window is "initially" displayed, and a click is the sitting's.
    let document = windowed();
    let mut view = ViewState::of(&document);
    view.set_popup_open(POPUP, true);
    assert!(window_open(&document, &view));
    assert_eq!(saved_open(&document, &view), Object::Null);
}

#[test]
fn a_script_retypes_a_text_note_and_its_window_shows_it() {
    let document = windowed();
    let runner = Arc::new(Queued::default());
    let mut view = ViewState::of(&document);
    view.run_scripts_with(Some(Arc::clone(&runner) as Arc<dyn ScriptRunner>));
    runner.queue(4, AnnotationChange::Contents("Retyped".to_owned()));
    runner.queue(6, AnnotationChange::Contents("Square".to_owned()));
    view.run_open_scripts(&document, 0);
    let page = pdf_model::Pages::new(&document).get(0).expect("page one");
    let windows = pdf_model::popup::popups(&document, &page, &view);
    assert_eq!(
        windows
            .iter()
            .find(|window| window.annotation == POPUP)
            .and_then(|window| window.text.as_deref()),
        Some("Retyped"),
        "§12.5.6.4: the window contains the text of the note"
    );
    // A square's `/Contents` is no reader's edit here, and the change says so.
    assert!(
        view.script_reports()
            .iter()
            .any(|report| report.contains("annotation 6 0")),
        "{:?}",
        view.script_reports()
    );
    let written = view.save(&document).expect("the update writes");
    let saved = Document::open(written.bytes).expect("the update reads back");
    assert_eq!(
        saved.get_key(&object(&saved, 4), "Contents"),
        Object::String(b"Retyped".to_vec().into())
    );
    assert_eq!(
        saved.get_key(&object(&saved, 6), "Contents"),
        Object::Null,
        "nothing is written for the square"
    );
}

#[test]
fn a_group_s_subordinate_note_is_not_retyped() {
    // §12.5.6.2: a subordinate's `Contents` is a group attribute, and "shall be ignored".
    let document = assembled(&[
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [4 0 R 5 0 R] >>",
        "<< /Type /Annot /Subtype /Text /Rect [0 0 20 20] /Contents (Primary) >>",
        "<< /Type /Annot /Subtype /Text /Rect [30 0 50 20] /Contents (Sub) /IRT 4 0 R \
         /RT /Group >>",
    ]);
    let mut view = ViewState::of(&document);
    let id = |number| ObjectId {
        number,
        generation: 0,
    };
    assert!(!view.set_note_text(&document, id(5), "No"));
    assert!(view.set_note_text(&document, id(4), "Yes"));
}

/// One page of seven squares whose Table 167 `/F` differ, for `getAnnots`'s flag filters.
fn flagged() -> Document {
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R /OpenAction << /S /JavaScript /JS (read\\(\\);) >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] \
         /Annots [4 0 R 5 0 R 6 0 R 7 0 R 8 0 R 9 0 R 10 0 R] >>",
        // No `/F` and no appearance stream: bit 3 is ignored, so it prints.
        "<< /Type /Annot /Subtype /Square /Rect [0 0 10 10] >>",
        // Print, with an appearance stream.
        "<< /Type /Annot /Subtype /Square /Rect [0 0 10 10] /F 4 /AP << /N 11 0 R >> >>",
        // No Print bit, with an appearance stream: never printed.
        "<< /Type /Annot /Subtype /Square /Rect [0 0 10 10] /F 0 /AP << /N 11 0 R >> >>",
        // NoView.
        "<< /Type /Annot /Subtype /Square /Rect [0 0 10 10] /F 32 >>",
        // NoView and ToggleNoView: seen under the pointer, and live for it.
        "<< /Type /Annot /Subtype /Square /Rect [0 0 10 10] /F 288 >>",
        // ReadOnly.
        "<< /Type /Annot /Subtype /Square /Rect [0 0 10 10] /F 64 >>",
        // Hidden and Print.
        "<< /Type /Annot /Subtype /Square /Rect [0 0 10 10] /F 6 /AP << /N 11 0 R >> >>",
        "<< /Type /XObject /Subtype /Form /BBox [0 0 10 10] /Length 0 >>\nstream\n\nendstream",
    ])
}

#[test]
fn each_annotation_is_told_where_table_167_lets_it_go() {
    let document = flagged();
    let runner = Arc::new(Queued::default());
    let mut view = ViewState::of(&document);
    view.run_scripts_with(Some(Arc::clone(&runner) as Arc<dyn ScriptRunner>));
    view.run_open_scripts(&document, 0);
    let told = runner.told.lock().expect("the record").clone();
    let reach: Vec<(u32, bool, bool, bool, bool)> = told
        .first()
        .expect("a telling")
        .iter()
        .map(|a| {
            (
                a.number,
                a.hidden,
                a.reach.printed,
                a.reach.viewed,
                a.reach.interactive,
            )
        })
        .collect();
    // `printed`, `viewed` and `interactive` set `Hidden` aside; the realm composes it.
    assert_eq!(
        reach,
        vec![
            (4, false, true, true, true),
            (5, false, true, true, true),
            (6, false, false, true, true),
            (7, false, true, false, false),
            (8, false, true, true, true),
            (9, false, true, true, false),
            (10, true, true, true, true),
        ]
    );
}
