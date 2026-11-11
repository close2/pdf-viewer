//! Tables 29, 197 and 198: real viewer action triggers with valid complete PDF fixtures.
#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "small fixed test fixtures and required observable events"
)]

use std::io::Write as _;
use std::sync::{Arc, Mutex};

use pdf_model::view::{ScriptEvent, ScriptResult, ScriptRunner, ScriptSite};
use pdf_syntax::Document;
use viewer_core::{
    Answer, Command, DocumentId, Event, PageTarget, PointerAction, Query, ScriptRunners, Scripting,
    Viewer,
};

const DOCUMENT: DocumentId = DocumentId(1);

fn assemble(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        bytes.extend_from_slice(object);
        bytes.extend_from_slice(b"\nendobj\n");
    }
    let xref = bytes.len();
    bytes.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        writeln!(&mut bytes, "{offset:010} 00000 n ").expect("Vec writes cannot fail");
    }
    bytes.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    bytes
}

fn open_uri(uri: &str) -> Vec<u8> {
    format!("<< /S /URI /URI ({uri}) >>").into_bytes()
}

fn uris(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::OpenUri { uri, .. } => Some(uri.clone()),
            _ => None,
        })
        .collect()
}

fn open(bytes: Vec<u8>) -> (Viewer, Vec<Event>) {
    let parsed = Document::open(bytes.clone()).expect("complete valid PDF fixture");
    assert!(!parsed.was_recovered(), "fixture has exact xref offsets");
    let mut viewer = Viewer::new(200, 200, 1.0);
    let events = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    (viewer, events)
}

fn two_page_open_action(action: &[u8]) -> Vec<u8> {
    assemble(&[
        b"<< /Type /Catalog /Pages 2 0 R /OpenAction 5 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>".to_vec(),
        action.to_vec(),
    ])
}

fn two_page_link_action(action: &[u8]) -> Vec<u8> {
    assemble(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Annots [6 0 R] >>"
            .to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>".to_vec(),
        action.to_vec(),
        b"<< /Type /Annot /Subtype /Link /Rect [10 10 30 30] /A 5 0 R >>".to_vec(),
    ])
}

fn current_page(viewer: &Viewer) -> usize {
    let Answer::Page { index, .. } = viewer.query(Query::CurrentPage) else {
        panic!("an opened viewer has a current page");
    };
    index
}

type ScriptRecord = Arc<Mutex<Vec<(ScriptSite, String)>>>;

#[derive(Debug)]
struct RecordingRunner(ScriptRecord);

impl ScriptRunner for RecordingRunner {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        self.0
            .lock()
            .expect("the test record is not poisoned")
            .push((event.site, event.script.to_owned()));
        ScriptResult {
            rc: true,
            value: None,
            change: None,
            edits: Vec::new(),
            report: Vec::new(),
        }
    }
}

#[derive(Debug)]
struct RecordingRunners(ScriptRecord);

impl ScriptRunners for RecordingRunners {
    fn runner(&self) -> Arc<dyn ScriptRunner> {
        Arc::new(RecordingRunner(Arc::clone(&self.0)))
    }
}

#[test]
fn catalog_open_action_executes_a_supported_non_goto_action() {
    // Controls first: catalog GoTo destinations are currently applied, and the catalog's
    // JavaScript action has its own first-present path. Neither should mask the generic Named case.
    let (goto, _) = open(two_page_open_action(b"<< /S /GoTo /D [4 0 R /Fit] >>"));
    assert_eq!(current_page(&goto), 1, "the OpenAction GoTo control works");

    // Prove this specific action is supported by the ordinary activation route before testing
    // whether the catalog trigger reaches that route.
    let (mut linked, _) = open(two_page_link_action(b"<< /S /Named /N /LastPage >>"));
    let at = page_point(&linked, [10.0, 10.0, 30.0, 30.0]);
    assert!(matches!(
        linked.query(Query::LinkAt(at)),
        Answer::Link(true)
    ));
    linked
        .handle(Command::Pointer {
            at,
            action: PointerAction::Pressed,
        })
        .for_each(drop);
    linked
        .handle(Command::Pointer {
            at,
            action: PointerAction::Released,
        })
        .for_each(drop);
    assert_eq!(current_page(&linked), 1, "the link's Named LastPage works");

    let scripts: ScriptRecord = Arc::default();
    let mut script_viewer = Viewer::new(200, 200, 1.0);
    script_viewer
        .handle(Command::Scripts(Scripting::Run(Arc::new(
            RecordingRunners(Arc::clone(&scripts)),
        ))))
        .for_each(drop);
    script_viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: two_page_open_action(b"<< /S /JavaScript /JS (void 0;) >>").into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    script_viewer.handle(Command::Presented).for_each(drop);
    assert_eq!(
        *scripts.lock().expect("the script record"),
        [(ScriptSite::OpenAction, "void 0;".to_owned())],
        "the catalog JavaScript OpenAction control runs"
    );

    // §12.6.1 / Table 29 admits an action dictionary here. /Named /LastPage is supported as a
    // viewer action elsewhere, so opening should perform it and present page two.
    let (mut viewer, _) = open(two_page_open_action(b"<< /S /Named /N /LastPage >>"));
    viewer.handle(Command::Presented).for_each(drop);
    assert_eq!(
        current_page(&viewer),
        1,
        "a catalog OpenAction action is performed when the document opens"
    );
}

fn page_point(viewer: &Viewer, rect: [f32; 4]) -> (f32, f32) {
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the first page has geometry");
    };
    let (x, y) = (
        f32::midpoint(rect[0], rect[2]),
        f32::midpoint(rect[1], rect[3]),
    );
    (
        geometry.origin.0 + x * geometry.scale,
        geometry.origin.1 + (100.0 - y) * geometry.scale,
    )
}

fn page_visibility_fixture(annotation_flags: i64) -> Vec<u8> {
    let page1 = format!(
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> \
         /Annots [4 0 R] /AA << /O {} /C {} >> >>",
        "<< /S /URI /URI (page-open) >>", "<< /S /URI /URI (page-close) >>"
    );
    let annotation = format!(
        "<< /Type /Annot /Subtype /Square /P 3 0 R /Rect [10 10 30 30] /F {annotation_flags} \
         /AA << /PO {} /PC {} /PV {} /PI {} >> >>",
        "<< /S /URI /URI (annotation-open) >>",
        "<< /S /URI /URI (annotation-close) >>",
        "<< /S /URI /URI (annotation-visible) >>",
        "<< /S /URI /URI (annotation-invisible) >>"
    );
    assemble(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R 5 0 R] /Count 2 >>".to_vec(),
        page1.into_bytes(),
        annotation.into_bytes(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>".to_vec(),
    ])
}

#[test]
fn hidden_and_no_view_flags_do_not_suppress_page_visibility_triggers() {
    // Positive control first: ordinary annotations take part in page open/close events.
    let (mut visible, opened) = open(page_visibility_fixture(0));
    assert_eq!(
        uris(&opened),
        ["page-open", "annotation-open", "annotation-visible"]
    );
    let turned: Vec<Event> = visible
        .handle(Command::GoTo(PageTarget::Index(1)))
        .collect();
    assert_eq!(
        uris(&turned),
        ["annotation-close", "annotation-invisible", "page-close"]
    );

    // Table 197 Note 1 says annotation flags do not affect these events. F=34 sets Hidden and
    // NoView, both of which suppress pointer interaction in other contexts.
    let (mut hidden, opened) = open(page_visibility_fixture(34));
    assert_eq!(
        uris(&opened),
        ["page-open", "annotation-open", "annotation-visible"],
        "the hidden/no-view annotation still receives PO and PV"
    );
    let turned: Vec<Event> = hidden.handle(Command::GoTo(PageTarget::Index(1))).collect();
    assert_eq!(
        uris(&turned),
        ["annotation-close", "annotation-invisible", "page-close"],
        "the hidden/no-view annotation still receives PC and PI"
    );
}

#[test]
fn entering_a_nested_annotation_exits_the_outer_annotation_first() {
    let (mut viewer, _) = open(assemble(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Annots [4 0 R 5 0 R] >>".to_vec(),
        b"<< /Type /Annot /Subtype /Square /P 3 0 R /Rect [10 10 90 90] /AA << /E 6 0 R /X 7 0 R >> >>".to_vec(),
        b"<< /Type /Annot /Subtype /Square /P 3 0 R /Rect [30 30 70 70] /AA << /E 8 0 R >> >>".to_vec(),
        open_uri("outer-enter"),
        open_uri("outer-exit"),
        open_uri("inner-enter"),
    ]));

    let outer = page_point(&viewer, [10.0, 10.0, 20.0, 20.0]);
    let inner = page_point(&viewer, [40.0, 40.0, 60.0, 60.0]);
    let entered: Vec<Event> = viewer
        .handle(Command::Pointer {
            at: outer,
            action: PointerAction::Moved,
        })
        .collect();
    assert_eq!(uris(&entered), ["outer-enter"]);

    let nested: Vec<Event> = viewer
        .handle(Command::Pointer {
            at: inner,
            action: PointerAction::Moved,
        })
        .collect();
    assert_eq!(
        uris(&nested),
        ["outer-exit", "inner-enter"],
        "the fourth mouse-event constraint requires X for the first active annotation"
    );
}
