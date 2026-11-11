//! Public-operation timing for catalog actions when save or print is held or refused.
//! XFA cleanup and catalog `DidSave` have distinct conditions (ADRs 1614 and 2763).

#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "test fixtures and the fixed R2 encryption vector must fail loudly if malformed"
)]

use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

use pdf_model::restriction::Operation;
use pdf_model::view::{
    DocumentTrigger, ScriptEdit, ScriptEvent, ScriptResult, ScriptRunner, ScriptSite,
};
use pdf_model::xfa::{Form, FormLanguage, Ran, Scalar, Step};
use viewer_core::{
    Command, DocumentId, Event, Printing, RestrictionLevel, RestrictionPolicy, RestrictionScope,
    ScriptRunners, Scripting, Sheet, Viewer,
};

const DOCUMENT: DocumentId = DocumentId(1);

#[derive(Debug, Default)]
struct Calls(Mutex<Vec<&'static str>>);

impl Calls {
    fn push(&self, call: &'static str) {
        self.0.lock().expect("single test thread").push(call);
    }

    fn snapshot(&self) -> Vec<&'static str> {
        self.0.lock().expect("single test thread").clone()
    }
}

#[derive(Debug)]
struct RecordingRunner(Arc<Calls>);

impl ScriptRunner for RecordingRunner {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        if matches!(event.site, ScriptSite::Document(_)) {
            assert_eq!(
                event.script, "void 0;",
                "catalog script bytes, including decryption"
            );
        }
        match event.site {
            ScriptSite::Document(DocumentTrigger::WillSave) => self.0.push("WS"),
            ScriptSite::Document(DocumentTrigger::DidSave) => self.0.push("DS"),
            ScriptSite::Document(DocumentTrigger::WillPrint) => self.0.push("WP"),
            ScriptSite::Document(DocumentTrigger::DidPrint) => self.0.push("DP"),
            _ => {}
        }
        ScriptResult {
            rc: true,
            value: None,
            change: None,
            edits: Vec::<ScriptEdit>::new(),
            report: Vec::new(),
        }
    }

    fn run_form_script(
        &self,
        form: &mut Form<'_>,
        node: usize,
        _language: FormLanguage,
        script: &str,
        _steps_left: u64,
    ) -> Ran {
        match script.trim() {
            "cancel" => {
                self.0.push("XFA-preSave");
                let set = Step::path("$event.cancelAction")
                    .and_then(|path| form.resolve(node, &path))
                    .and_then(|nodes| {
                        let flag = *nodes.first().ok_or("no action is under way")?;
                        form.set(flag, Scalar::Number(1.0))
                    });
                Ran {
                    steps: 1,
                    outcome: set.map(|()| Scalar::Number(1.0)),
                }
            }
            "9" => {
                self.0.push("XFA-postSave");
                Ran {
                    steps: 1,
                    outcome: form
                        .set(node, Scalar::Number(9.0))
                        .map(|()| Scalar::Number(9.0)),
                }
            }
            other => Ran {
                steps: 0,
                outcome: Err(format!("unexpected test script {other:?}")),
            },
        }
    }
}

#[derive(Debug)]
struct Runners(Arc<RecordingRunner>);

impl ScriptRunners for Runners {
    fn runner(&self) -> Arc<dyn ScriptRunner> {
        self.0.clone()
    }
}

fn open_with_runner(bytes: Vec<u8>, calls: Arc<Calls>, save_level: RestrictionLevel) -> Viewer {
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer
        .handle(Command::Restrict(RestrictionScope::Window(
            RestrictionPolicy::default().with(Operation::Save, save_level),
        )))
        .for_each(drop);
    viewer
        .handle(Command::Scripts(Scripting::Run(Arc::new(Runners(
            Arc::new(RecordingRunner(calls)),
        )))))
        .for_each(drop);
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
        "complete fixture opens as one page"
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::OpenFailed { .. })),
        "fixture is not recovered through an open failure"
    );
    viewer.handle(Command::Presented).for_each(drop);
    viewer
}

fn hybrid_with_document_actions() -> Vec<u8> {
    let template = "<template xmlns=\"http://www.xfa.org/schema/xfa-template/3.3/\"><subform name=\"form1\"><field name=\"Total\"><ui><textEdit/></ui><value><text/></value><event activity=\"preSave\" ref=\"$host\"><script>cancel</script></event><event activity=\"postSave\" ref=\"$host\"><script>9</script></event></field></subform></template>";
    let datasets = "<xfa:datasets xmlns:xfa=\"http://www.xfa.org/schema/xfa-data/1.0/\"><xfa:data><form1><Total/></form1></xfa:data></xfa:datasets>";
    let stream = |text: &str| format!("<< /Length {} >>\nstream\n{text}\nendstream", text.len());
    let appearance = "q 1 1 1 rg 0 0 200 30 re f Q";
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R /AA << /WS << /S /JavaScript /JS (void 0;) >> /DS << /S /JavaScript /JS (void 0;) >> >> /AcroForm 4 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Resources << /Font << /Helv 8 0 R >> >> /Annots [7 0 R] >>".to_owned(),
        "<< /Fields [9 0 R] /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv 8 0 R >> >> /XFA [(template) 5 0 R (datasets) 6 0 R] >>".to_owned(),
        stream(template),
        stream(datasets),
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (Total[0]) /Parent 9 0 R /P 3 0 R /F 4 /Rect [20 20 220 50] /DA (/Helv 12 Tf 0 g) /AP << /N 10 0 R >> >>".to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_owned(),
        "<< /T (form1[0]) /Kids [7 0 R] >>".to_owned(),
        format!("<< /Type /XObject /Subtype /Form /BBox [0 0 200 30] /Resources << /Font << /Helv 8 0 R >> >> /Length {} >>\nstream\n{appearance}\nendstream", appearance.len()),
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
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<0123456789abcdef0123456789abcdef> <fedcba9876543210fedcba9876543210>] >>\nstartxref\n{xref}\n%%EOF\n"
    );
    pdf.into_bytes()
}

fn has_saved(events: &[Event]) -> bool {
    events
        .iter()
        .any(|event| matches!(event, Event::Saved { .. }))
}

/// Catalog /WS is before XFA preSave; catalog /DS is called only after an actual write.
/// XFA postSave is distinct: XFA cleanup runs after a cancelled save even when no `Event::Saved`
/// exists, whereas Table 200 /DS must remain absent on refusal or an Ask/no answer.
#[test]
fn catalog_save_triggers_follow_xfa_cancellation_and_public_save_outcome() {
    let calls = Arc::new(Calls::default());
    let mut viewer = open_with_runner(
        hybrid_with_document_actions(),
        calls.clone(),
        RestrictionLevel::Off,
    );
    let off_events: Vec<Event> = viewer.handle(Command::Save).collect();
    assert!(has_saved(&off_events), "Off permits the cancelled save");
    assert_eq!(
        calls.snapshot(),
        ["WS", "XFA-preSave", "DS", "XFA-postSave"]
    );

    for (level, answer, should_write) in [
        (RestrictionLevel::On, None, false),
        (RestrictionLevel::Ask, Some(false), false),
        (RestrictionLevel::Ask, Some(true), true),
    ] {
        let calls = Arc::new(Calls::default());
        let mut viewer = open_with_runner(hybrid_with_document_actions(), calls.clone(), level);
        let first: Vec<Event> = viewer.handle(Command::Save).collect();
        if level == RestrictionLevel::Ask {
            assert!(!has_saved(&first), "Ask holds the save before the answer");
            assert!(first.iter().any(|event| matches!(
                event,
                Event::Asking {
                    operation: Operation::Save,
                    ..
                }
            )));
            assert_eq!(calls.snapshot(), ["WS", "XFA-preSave"]);
            let answered: Vec<Event> = viewer
                .handle(Command::Answer {
                    document: DOCUMENT,
                    proceed: answer.expect("Ask has an answer"),
                })
                .collect();
            assert_eq!(has_saved(&answered), should_write);
        } else {
            assert!(!has_saved(&first), "On refuses the cancelled save");
            assert!(first.iter().any(|event| matches!(
                event,
                Event::Refused {
                    operation: Operation::Save,
                    ..
                }
            )));
        }
        let trace = calls.snapshot();
        assert!(
            trace.iter().position(|call| *call == "WS").expect("WS ran")
                < trace
                    .iter()
                    .position(|call| *call == "XFA-preSave")
                    .expect("XFA preSave ran")
        );
        assert_eq!(
            trace.contains(&"DS"),
            should_write,
            "DS iff a file was actually saved"
        );
        assert!(
            trace.contains(&"XFA-postSave"),
            "XFA cleanup fires after cancellation or a write"
        );
        if should_write {
            assert_eq!(trace, ["WS", "XFA-preSave", "DS", "XFA-postSave"]);
        } else {
            assert_eq!(trace, ["WS", "XFA-preSave", "XFA-postSave"]);
        }
    }
}

/// A valid PDF 1.7 Standard Security Handler revision-2 fixture with print permission bit 3
/// clear. The fixed /O, /U and encrypted JavaScript strings are produced by §7.6.4.3.2 and
/// §7.6.4.4.2–4; object 1's two /JS values use its object key. Both trailer IDs are 16 bytes.
fn encrypted_print_actions() -> Vec<u8> {
    let bodies = [
        "<< /Type /Catalog /Pages 2 0 R /AA << /WP << /S /JavaScript /JS <AB25BB6BAFB2FB> >> /DP << /S /JavaScript /JS <AB25BB6BAFB2FB> >> >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources <<>> /Contents 4 0 R >>".to_owned(),
        "<< /Length 0 >>\nstream\nendstream".to_owned(),
        "<< /Filter /Standard /V 1 /R 2 /Length 40 /O <C92422687FACEE686E373F10B5C7D04738053152F7E2EE30E11C69EC442576AB> /U <6821E85F3DD8D5EA2E8B639606148CDB9C4AAF4E32E980300FE45FFFA1945795> /P -8 >>".to_owned(),
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
        "trailer\n<< /Size {size} /Root 1 0 R /Encrypt 5 0 R /ID [<747269676765722D74696D696E673031> <747269676765722D74696D696E673032>] >>\nstartxref\n{xref}\n%%EOF\n"
    );
    pdf.into_bytes()
}

fn print_viewer(calls: Arc<Calls>, level: RestrictionLevel) -> Viewer {
    let mut viewer = Viewer::new(200, 200, 1.0);
    viewer
        .handle(Command::Restrict(RestrictionScope::Window(
            RestrictionPolicy::default().with(Operation::Print, level),
        )))
        .for_each(drop);
    viewer
        .handle(Command::Scripts(Scripting::Run(Arc::new(Runners(
            Arc::new(RecordingRunner(calls)),
        )))))
        .for_each(drop);
    let events: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: encrypted_print_actions().into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Opened { pages: 1, .. })),
        "R2 fixture decrypts and opens"
    );
    viewer
}

fn sheet() -> Sheet {
    Sheet {
        media: None,
        scale: 1.0,
        page_scale: 1.0,
    }
}

/// Table 200 /WP and /DP run only for a print job admitted under the reader's configured level.
/// `Ask` defers both callbacks until yes; no answer or a no answer runs neither.
#[test]
fn print_triggers_are_absent_until_a_permission_denied_job_is_admitted() {
    let cases = [
        (RestrictionLevel::Off, None, true),
        (RestrictionLevel::On, None, false),
        (RestrictionLevel::Ask, Some(false), false),
        (RestrictionLevel::Ask, Some(true), true),
    ];
    for (level, answer, admitted) in cases {
        let calls = Arc::new(Calls::default());
        let mut viewer = print_viewer(calls.clone(), level);
        let started: Vec<Event> = viewer
            .handle(Command::Print(Printing::Start(sheet())))
            .collect();
        if level == RestrictionLevel::Ask {
            assert!(started.iter().any(|event| matches!(
                event,
                Event::Asking {
                    operation: Operation::Print,
                    ..
                }
            )));
            assert!(
                calls.snapshot().is_empty(),
                "no callbacks before permission answer"
            );
            let answered: Vec<Event> = viewer
                .handle(Command::Answer {
                    document: DOCUMENT,
                    proceed: answer.expect("Ask has an answer"),
                })
                .collect();
            assert_eq!(
                answered
                    .iter()
                    .any(|event| matches!(event, Event::Printing { .. })),
                admitted
            );
        } else {
            assert_eq!(
                started
                    .iter()
                    .any(|event| matches!(event, Event::Printing { .. })),
                admitted
            );
        }
        assert_eq!(
            calls.snapshot().contains(&"WP"),
            admitted,
            "WP follows admission"
        );
        if admitted {
            viewer
                .handle(Command::Print(Printing::Finish))
                .for_each(drop);
            assert_eq!(
                calls.snapshot(),
                ["WP", "DP"],
                "one admitted job has one pair"
            );
        } else {
            viewer
                .handle(Command::Print(Printing::Finish))
                .for_each(drop);
            assert!(
                calls.snapshot().is_empty(),
                "no job means neither WP nor DP"
            );
        }
    }
}
