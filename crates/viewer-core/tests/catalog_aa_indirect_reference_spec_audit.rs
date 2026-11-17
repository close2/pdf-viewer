//! Table 29 `/AA` and its `/WS` trigger accept equivalent indirect objects.
//! The public save operation dispatches the exact script to a recording runner;
//! a no-AA control does not. This checks dispatch, not JavaScript execution.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "the exact-xref fixture and public operation are the assertion"
)]

use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

use pdf_model::view::{DocumentTrigger, ScriptEvent, ScriptResult, ScriptRunner, ScriptSite};
use viewer_core::{Command, DocumentId, Event, ScriptRunners, Scripting, Viewer};

const DOC: DocumentId = DocumentId(1);

#[derive(Debug, Default)]
struct Recorder(Mutex<Vec<ScriptSite>>);
impl ScriptRunner for Recorder {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        assert_eq!(
            event.script, "var marker = 'catalogWS';",
            "the selected action supplies its own script"
        );
        self.0.lock().expect("single test thread").push(event.site);
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
struct Runner(Arc<Recorder>);
impl ScriptRunners for Runner {
    fn runner(&self) -> Arc<dyn ScriptRunner> {
        self.0.clone()
    }
}

/// Exact classic-xref PDF. The indirect variant puts both catalog /AA and /WS in separate objects.
fn fixture(indirect: Option<bool>) -> Vec<u8> {
    let catalog = if indirect == Some(true) {
        "<< /Type /Catalog /Pages 2 0 R /AA 4 0 R >>"
    } else if indirect == Some(false) {
        "<< /Type /Catalog /Pages 2 0 R /AA << /WS << /S /JavaScript /JS (var marker = 'catalogWS';) >> >> >>"
    } else {
        "<< /Type /Catalog /Pages 2 0 R >>"
    };
    let mut bodies = vec![
        catalog.to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>".to_owned(),
    ];
    if indirect == Some(true) {
        bodies.push("<< /WS 5 0 R >>".to_owned());
        bodies.push("<< /S /JavaScript /JS (var marker = 'catalogWS';) >>".to_owned());
    }
    let mut pdf = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(pdf.len());
        let _ = writeln!(pdf, "{} 0 obj\n{body}\nendobj", index + 1);
    }
    let xref = pdf.len();
    let size = bodies.len() + 1;
    let _ = writeln!(pdf, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(pdf, "{offset:010} 00000 n ");
    }
    let _ = write!(
        pdf,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    pdf.into_bytes()
}

fn observed_sites(bytes: Vec<u8>) -> Vec<ScriptSite> {
    let document = pdf_syntax::Document::open(bytes.clone()).expect("valid exact-xref PDF");
    assert!(!document.was_recovered(), "fixture must use its exact xref");
    let recorder = Arc::new(Recorder::default());
    let mut viewer = Viewer::new(100, 100, 1.0);
    viewer
        .handle(Command::Scripts(Scripting::Run(Arc::new(Runner(
            recorder.clone(),
        )))))
        .for_each(drop);
    let opened: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOC,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        opened
            .iter()
            .any(|event| matches!(event, Event::Opened { document, pages: 1 } if *document == DOC))
    );
    viewer.handle(Command::Presented).for_each(drop);
    let saved: Vec<Event> = viewer.handle(Command::Save).collect();
    assert!(
        saved
            .iter()
            .any(|event| matches!(event, Event::Saved { .. })),
        "save completes"
    );
    recorder.0.lock().expect("single test thread").clone()
}

#[test]
fn catalog_aa_and_trigger_value_accept_direct_and_indirect_objects() {
    assert!(
        observed_sites(fixture(None)).is_empty(),
        "no AA means no script dispatch"
    );
    let expected = vec![ScriptSite::Document(DocumentTrigger::WillSave)];
    assert_eq!(
        observed_sites(fixture(Some(false))),
        expected,
        "direct control reaches /WS"
    );
    assert_eq!(
        observed_sites(fixture(Some(true))),
        expected,
        "indirect /AA and /WS reach the same public trigger"
    );
}
