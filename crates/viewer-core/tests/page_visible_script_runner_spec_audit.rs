//! Public-runner regression for Table 197's visibility events.
//!
//! This records the site handed to the reader's existing `ScriptRunner` seam. It does not evaluate
//! JavaScript or claim anything about an engine. A two-page layout makes /PV distinct from /PO:
//! page 2 becomes visible while page 1 remains current, and only later becomes current itself.

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "small exact-xref test fixture"
)]

use std::io::Write as _;
use std::sync::{Arc, Mutex, PoisonError};

use pdf_model::action::Trigger;
use pdf_model::view::{ScriptEvent, ScriptResult, ScriptRunner, ScriptSite};
use pdf_model::viewer_preferences::PageLayout;
use pdf_syntax::Document;
use viewer_core::{
    Answer, Command, DocumentId, PageTarget, Query, ScriptRunners, Scripting, Viewer, Zoom,
};

const DOCUMENT: DocumentId = DocumentId(1);
type Handed = Arc<Mutex<Vec<(ScriptSite, String)>>>;

fn fixture() -> Vec<u8> {
    let js = |source: &str| format!("<< /S /JavaScript /JS ({source}) >>");
    let objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R 5 0 R] /Count 2 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Annots [4 0 R] >>"
            .to_vec(),
        format!(
            "<< /Type /Annot /Subtype /Square /P 3 0 R /Rect [10 10 30 30] /F 0 \
             /AA << /PO {} /PC {} /PV {} /PI {} >> >>",
            js("var marker = 'page0-open';"),
            js("var marker = 'page0-close';"),
            js("var marker = 'page0-visible';"),
            js("var marker = 'page0-invisible';"),
        )
        .into_bytes(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Annots [6 0 R] >>"
            .to_vec(),
        format!(
            "<< /Type /Annot /Subtype /Square /P 5 0 R /Rect [10 10 30 30] /F 0 \
             /AA << /PO {} /PV {} /PI {} >> >>",
            js("var marker = 'page1-open';"),
            js("var marker = 'page1-visible';"),
            js("var marker = 'page1-invisible';"),
        )
        .into_bytes(),
    ];

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

#[derive(Debug)]
struct Recording(Handed);

impl ScriptRunner for Recording {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
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
struct Recorders(Handed);

impl ScriptRunners for Recorders {
    fn runner(&self) -> Arc<dyn ScriptRunner> {
        Arc::new(Recording(Arc::clone(&self.0)))
    }
}

fn scripts(handed: &Handed) -> Vec<(ScriptSite, String)> {
    std::mem::take(&mut *handed.lock().unwrap_or_else(PoisonError::into_inner))
}

#[test]
fn visible_page_runner_site_is_timed_to_visibility_not_open_or_current_page() {
    let (mut viewer, handed) = page_zero_opened_with_runner();
    put_page_one_beside_current(&mut viewer);
    assert_eq!(
        scripts(&handed),
        vec![(
            ScriptSite::Annotation(Trigger::PageVisible),
            "var marker = 'page1-visible';".to_owned()
        )],
        "page one became visible beside the current page; its PV must not wait for PO"
    );
}

/// The runner control is current page zero, which is opened and visible in the initial layout.
fn page_zero_opened_with_runner() -> (Viewer, Handed) {
    let bytes = fixture();
    let parsed = Document::open(bytes.clone()).expect("complete PDF fixture");
    assert!(!parsed.was_recovered(), "fixture has exact xref offsets");
    assert!(!parsed.cross_reference_chain_was_cut());

    let handed = Handed::default();
    let mut viewer = Viewer::new(800, 1000, 1.0);
    viewer
        .handle(Command::Scripts(Scripting::Run(Arc::new(Recorders(
            Arc::clone(&handed),
        )))))
        .for_each(drop);
    viewer
        .handle(Command::Layout(PageLayout::SinglePage))
        .for_each(drop);
    viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer.handle(Command::Presented).for_each(drop);
    assert_eq!(
        scripts(&handed),
        vec![
            (
                ScriptSite::Annotation(Trigger::PageOpen),
                "var marker = 'page0-open';".to_owned(),
            ),
            (
                ScriptSite::Annotation(Trigger::PageVisible),
                "var marker = 'page0-visible';".to_owned(),
            ),
        ],
        "positive control: current page receives its distinct open/visible sites"
    );
    (viewer, handed)
}

fn put_page_one_beside_current(viewer: &mut Viewer) {
    viewer
        .handle(Command::Zoom {
            zoom: Zoom::Scale(0.5),
            at: None,
        })
        .for_each(drop);
    viewer
        .handle(Command::Layout(PageLayout::TwoPageLeft))
        .for_each(drop);
    assert!(matches!(
        viewer.query(Query::CurrentPage),
        Answer::Page { index: 0, .. }
    ));
    assert!(matches!(
        viewer.query(Query::PageGeometry(1)),
        Answer::Geometry(_)
    ));
}

#[test]
fn page_still_visible_after_current_page_changes_gets_no_pi_or_duplicate_pv() {
    let (mut viewer, handed) = page_zero_opened_with_runner();
    put_page_one_beside_current(&mut viewer);
    // A corrected implementation may already have run page one's PV here; this test is an
    // independent assertion about what happens when current-page status changes afterward.
    let _visible_transition = scripts(&handed);

    viewer
        .handle(Command::GoTo(PageTarget::Index(1)))
        .for_each(drop);
    assert!(matches!(
        viewer.query(Query::CurrentPage),
        Answer::Page { index: 1, .. }
    ));
    assert!(matches!(
        viewer.query(Query::PageGeometry(0)),
        Answer::Geometry(_)
    ));
    assert!(matches!(
        viewer.query(Query::PageGeometry(1)),
        Answer::Geometry(_)
    ));
    assert_eq!(
        scripts(&handed),
        vec![
            (
                ScriptSite::Annotation(Trigger::PageClose),
                "var marker = 'page0-close';".to_owned(),
            ),
            (
                ScriptSite::Annotation(Trigger::PageOpen),
                "var marker = 'page1-open';".to_owned(),
            ),
        ],
        "page zero is closed but remains visible; page one is opened but was already visible"
    );
}

#[test]
fn page_that_leaves_the_arrangement_gets_pi_at_the_visibility_transition() {
    let (mut viewer, handed) = page_zero_opened_with_runner();
    put_page_one_beside_current(&mut viewer);
    // Drain any correctly timed PV from making page one visible; the assertion below concerns
    // the independent transition when it leaves the arrangement.
    let _visible_transition = scripts(&handed);

    viewer
        .handle(Command::Layout(PageLayout::SinglePage))
        .for_each(drop);
    assert!(matches!(
        viewer.query(Query::CurrentPage),
        Answer::Page { index: 0, .. }
    ));
    assert!(matches!(
        viewer.query(Query::PageGeometry(0)),
        Answer::Geometry(_)
    ));
    assert!(matches!(viewer.query(Query::PageGeometry(1)), Answer::None));
    assert_eq!(
        scripts(&handed),
        vec![(
            ScriptSite::Annotation(Trigger::PageInvisible),
            "var marker = 'page1-invisible';".to_owned(),
        )],
        "page one left the visible arrangement although it never became current"
    );
}
