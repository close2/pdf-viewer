//! Public Viewer save, close and print event timing, plus the project rc=false policy.

#![expect(
    clippy::panic,
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "a malformed fixture or missing operation result must fail the regression"
)]

use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

use pdf_render::Rasterizer as _;
use render_cpu::CpuRasterizer;

use pdf_model::view::{
    DocumentTrigger, ScriptEdit, ScriptEvent, ScriptResult, ScriptRunner, ScriptSite,
};
use viewer_core::{
    Answer, Command, DocumentId, Event, Printing, Query, ScriptRunners, Scripting, Sheet, Viewer,
};

const DOCUMENT: DocumentId = DocumentId(1);

/// One complete PDF 1.7 with a text widget and all cross-reference offsets accounted for.
fn fixture() -> Vec<u8> {
    let catalog = "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] \
        /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv 5 0 R >> >> >> \
        /AA << /WC << /S /JavaScript /JS (void 0;) >> \
        /WS << /S /JavaScript /JS (void 0;) >> \
        /DS << /S /JavaScript /JS (void 0;) >> \
        /WP << /S /JavaScript /JS (void 0;) >> \
        /DP << /S /JavaScript /JS (void 0;) >> >> >>";
    let bodies = [
        catalog.to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] \
          /Resources << /Font << /Helv 5 0 R >> >> /Annots [4 0 R] >>"
            .to_owned(),
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Stamp) /F 4 \
          /Rect [20 20 180 50] /DA (/Helv 12 Tf 0 g) >>"
            .to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
    ];
    let mut pdf = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in bodies.iter().enumerate() {
        offsets.push(pdf.len());
        let _ = write!(pdf, "{} 0 obj\n{body}\nendobj\n", index + 1);
    }
    let xref = pdf.len();
    let size = bodies.len() + 1;
    let _ = write!(pdf, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(pdf, "{offset:010} 00000 n ");
    }
    let _ = write!(
        pdf,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    pdf.into_bytes()
}

#[derive(Debug, Default)]
struct Recorder {
    sites: Mutex<Vec<ScriptSite>>,
}

impl ScriptRunner for Recorder {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        self.sites
            .lock()
            .expect("single test thread")
            .push(event.site);
        let edits = match event.site {
            ScriptSite::Document(DocumentTrigger::WillSave) => vec![ScriptEdit::Value {
                field: "Stamp".to_owned(),
                value: "written by WS".to_owned(),
            }],
            ScriptSite::Document(DocumentTrigger::DidSave) => vec![ScriptEdit::Value {
                field: "Stamp".to_owned(),
                value: "after DS".to_owned(),
            }],
            ScriptSite::Document(DocumentTrigger::WillPrint) => vec![ScriptEdit::Value {
                field: "Stamp".to_owned(),
                value: "during print".to_owned(),
            }],
            ScriptSite::Document(DocumentTrigger::DidPrint) => vec![ScriptEdit::Value {
                field: "Stamp".to_owned(),
                value: "after DP".to_owned(),
            }],
            _ => Vec::new(),
        };
        let report = match event.site {
            ScriptSite::Document(DocumentTrigger::WillClose) => vec!["WC marker".to_owned()],
            _ => Vec::new(),
        };
        ScriptResult {
            rc: false,
            value: None,
            change: None,
            edits,
            report,
        }
    }
}

#[derive(Debug)]
struct OneRunner(Arc<Recorder>);

impl ScriptRunners for OneRunner {
    fn runner(&self) -> Arc<dyn ScriptRunner> {
        self.0.clone()
    }
}

fn open(bytes: Vec<u8>, scripting: Scripting) -> Viewer {
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer.handle(Command::Scripts(scripting)).for_each(drop);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        events.iter().any(
            |event| matches!(event, Event::Opened { document, pages: 1 } if *document == DOCUMENT)
        ),
        "the complete fixture must open as one page"
    );
    viewer.handle(Command::Presented).for_each(drop);
    viewer
}

fn value(viewer: &Viewer) -> String {
    let Answer::Fields(fields) = viewer.query(Query::Fields) else {
        panic!("the complete form fixture answers its fields");
    };
    fields
        .into_iter()
        .find(|field| field.name.qualified == "Stamp")
        .and_then(|field| field.value)
        .map(|shown| shown.text)
        .unwrap_or_default()
}

/// Compare save and close triggers with public Viewer results: WS edits serialize, DS edits remain
/// live-only, and WC reports before Closed.
#[test]
fn catalog_document_triggers_bracket_public_viewer_operations() {
    let recorder = Arc::new(Recorder::default());
    let mut viewer = open(
        fixture(),
        Scripting::Run(Arc::new(OneRunner(recorder.clone()))),
    );

    let saved_events: Vec<Event> = viewer.handle(Command::Save).collect();
    let saved_bytes = saved_events.iter().find_map(|event| match event {
        Event::Saved { bytes, .. } => Some(bytes.clone()),
        _ => None,
    });
    let saved_bytes = saved_bytes.expect("a normal public save writes a file");
    assert_eq!(
        value(&viewer),
        "after DS",
        "DS follows the successful write"
    );
    let reopened = open(saved_bytes, Scripting::Off);
    assert_eq!(
        value(&reopened),
        "written by WS",
        "the serialized output contains the pre-save edit and not the post-save edit"
    );

    let closed: Vec<Event> = viewer.handle(Command::Close(DOCUMENT)).collect();
    let report_index = closed
        .iter()
        .position(|event| matches!(event, Event::Reported { notes, .. } if notes.iter().any(|note| note.ends_with("WC marker"))))
        .expect("the WC runner's observable report is emitted");
    let close_index = closed
        .iter()
        .position(|event| matches!(event, Event::Closed(id) if *id == DOCUMENT))
        .expect("the document closes despite rc=false");
    assert!(report_index < close_index, "WC is observed before Closed");

    let sites = recorder.sites.lock().expect("single test thread").clone();
    let document_sites: Vec<DocumentTrigger> = sites
        .into_iter()
        .filter_map(|site| match site {
            ScriptSite::Document(trigger) => Some(trigger),
            _ => None,
        })
        .collect();
    assert_eq!(
        document_sites,
        [
            DocumentTrigger::WillSave,
            DocumentTrigger::DidSave,
            DocumentTrigger::WillClose,
        ],
        "the save and close catalog actions are delivered in operation order despite rc=false"
    );
}

fn printed_ink(viewer: &Viewer) -> usize {
    let Answer::PrintPage(page) = viewer.query(Query::PrintPage(0)) else {
        panic!("an active print job exposes its actual page");
    };
    assert!(
        page.reports.is_empty(),
        "valid printable fixture: {:?}",
        page.reports
    );
    let image = CpuRasterizer::new()
        .with_medium(pdf_render::Medium::NONE)
        .rasterize(&page.list, page.target.expect("small printable page"))
        .expect("CPU draws the actual print page");
    image.data.chunks_exact(4).filter(|p| p[3] > 0).count()
}

/// Compare print triggers with the public print lifecycle, including a finish without a job.
#[test]
fn print_triggers_follow_a_successful_start_and_active_finish() {
    let recorder = Arc::new(Recorder::default());
    let mut viewer = open(
        fixture(),
        Scripting::Run(Arc::new(OneRunner(recorder.clone()))),
    );

    viewer
        .handle(Command::Print(Printing::Finish))
        .for_each(drop);
    assert!(
        recorder
            .sites
            .lock()
            .expect("single test thread")
            .is_empty()
    );

    let mut control = open(fixture(), Scripting::Off);
    control
        .handle(Command::Print(Printing::Start(Sheet {
            media: None,
            scale: 1.0,
            page_scale: 1.0,
        })))
        .for_each(drop);
    let blank_ink = printed_ink(&control);
    assert_eq!(
        blank_ink, 0,
        "without the WP edit the form has no printable marks"
    );

    let started: Vec<Event> = viewer
        .handle(Command::Print(Printing::Start(Sheet {
            media: None,
            scale: 1.0,
            page_scale: 1.0,
        })))
        .collect();
    assert!(
        started.iter().any(
            |event| matches!(event, Event::Printing { document, .. } if *document == DOCUMENT)
        )
    );
    assert_eq!(value(&viewer), "during print");
    assert!(
        printed_ink(&viewer) > 20,
        "the WP field edit reaches the page handed to the printer"
    );

    viewer
        .handle(Command::Print(Printing::Finish))
        .for_each(drop);
    assert_eq!(value(&viewer), "after DP");
    assert!(
        matches!(viewer.query(Query::PrintPage(0)), Answer::None),
        "Finish ends the actual print job"
    );
    let after_finish = recorder.sites.lock().expect("single test thread").clone();
    assert_eq!(
        after_finish,
        [
            ScriptSite::Document(DocumentTrigger::WillPrint),
            ScriptSite::Document(DocumentTrigger::DidPrint),
        ]
    );
    viewer
        .handle(Command::Print(Printing::Finish))
        .for_each(drop);
    assert_eq!(
        recorder.sites.lock().expect("single test thread").len(),
        2,
        "finishing without an active job is not a second completed print"
    );
}
