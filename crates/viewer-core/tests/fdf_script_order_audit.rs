//! Table 248 FDF document scripts through the public import-data route.
#![expect(
    clippy::expect_used,
    clippy::arithmetic_side_effects,
    reason = "complete source PDF/FDF fixtures and named test preconditions"
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::{Arc, Mutex, PoisonError};

use pdf_model::view::{ScriptEvent, ScriptResult, ScriptRunner};
use pdf_syntax::ObjectId;
use viewer_core::{Command, DocumentId, Event, Purpose, ScriptRunners, Scripting, Viewer};

const DOCUMENT: DocumentId = DocumentId(44);

#[derive(Debug, Clone, PartialEq, Eq)]
struct Seen {
    script: String,
    fields: Vec<(String, String)>,
}

#[derive(Debug, Default)]
struct Recorded {
    calls: Vec<Seen>,
    current_fields: BTreeMap<String, String>,
}

type Log = Arc<Mutex<Recorded>>;

#[derive(Debug)]
struct Recording(Log);

impl ScriptRunner for Recording {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        let mut record = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        for field in event.fields {
            let _: Option<String> = record
                .current_fields
                .insert(field.name.clone(), field.value.clone());
        }
        let fields = record
            .current_fields
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect();
        record.calls.push(Seen {
            script: event.script.to_owned(),
            fields,
        });
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
struct Recorders(Log);

impl ScriptRunners for Recorders {
    fn runner(&self) -> Arc<dyn ScriptRunner> {
        Arc::new(Recording(Arc::clone(&self.0)))
    }
}

fn file(header: &str, root: &str, objects: &[String]) -> Vec<u8> {
    let mut out = String::from(header);
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = writeln!(out, "{} 0 obj\n{body}\nendobj", index + 1);
    }
    let xref_at = out.len();
    let size = objects.len() + 1;
    let _ = writeln!(out, "xref\n0 {size}\n0000000000 65535 f ");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root {root} /ID [<30313233343536373839616263646566> <66656463626139383736353433323130>] >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    let bytes = out.into_bytes();
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("complete fixture");
    assert!(!parsed.was_recovered(), "exact source/FDF xref");
    bytes
}

fn source_pdf() -> Vec<u8> {
    let objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /Names << /JavaScript 7 0 R >> /OpenAction << /S /JavaScript /JS (/* open control */ void 0;) >> /AcroForm << /Fields [4 0 R] /NeedAppearances true /DA (/Helv 10 Tf 0 g) /DR << /Font << /Helv 6 0 R >> >> >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Resources << >> /Annots [4 0 R 5 0 R] >>".to_owned(),
        "<< /Type /Annot /Subtype /Widget /Rect [20 100 250 130] /F 4 /FT /Tx /T (name) /V (old) /DA (/Helv 10 Tf 0 g) >>".to_owned(),
        "<< /Type /Annot /Subtype /Link /Rect [20 20 120 50] /F 4 /A << /S /ImportData /F (answer.fdf) >> >>".to_owned(),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_owned(),
        "<< /Names [(existing) 8 0 R] >>".to_owned(),
        "<< /S /JavaScript /JS (/* existing global control */ void 0;) >>".to_owned(),
    ];
    file("%PDF-1.7\n", "1 0 R", &objects)
}

fn fdf() -> Vec<u8> {
    // /Doc is name/script pairs, not string/string pairs. The four unique script strings let the
    // recording runner distinguish each Table 248 timing point without evaluating JS semantics.
    let objects = vec![
        "<< /Version /1.7 /FDF << /Fields [<< /T (name) /V (imported) >>] /JavaScript << /Doc [/fromFdf (/* Doc function */ function fdfHelper(){return 1;})] /Before (/* before import */ void 0;) /After (/* after import */ void 0;) /AfterPermsReady (/* after permissions ready */ void 0;) >> >> >>".to_owned(),
    ];
    file("%FDF-1.2\n", "1 0 R", &objects)
}

fn value_for<'a>(seen: &'a Seen, field_name: &str) -> Option<&'a str> {
    seen.fields
        .iter()
        .find(|(name, _)| name == field_name)
        .map(|(_, value)| value.as_str())
}

fn notes(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Reported { notes, .. } => Some(notes.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

/// With script execution enabled, Table 248 adds Doc scripts before Before, executes Before while
/// the old target field is still present, and runs After only after the imported value is active.
/// `AfterPermsReady` also must follow import; its separate rights-ready timing is not observable in
/// `ScriptEvent` today and remains an explicit coverage limit rather than an assumed success.
#[test]
fn fdf_doc_before_after_and_after_perms_scripts_reach_the_import_runner_in_order() {
    let log = Log::default();
    let mut viewer = Viewer::new(600, 400, 1.0);
    viewer
        .handle(Command::Scripts(Scripting::Run(Arc::new(Recorders(
            Arc::clone(&log),
        )))))
        .for_each(drop);

    let opened: Vec<Event> = viewer
        .handle(Command::Open {
            id: DOCUMENT,
            bytes: source_pdf().into(),
            password: None,
            fragment: None,
        })
        .collect();
    assert!(
        !opened.is_empty(),
        "the complete source PDF opens before the FDF route is exercised"
    );
    viewer.handle(Command::Presented).for_each(drop);
    let controls = log
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .calls
        .clone();
    assert!(
        controls
            .iter()
            .any(|seen| seen.script == "/* open control */ void 0;"),
        "positive control: ordinary catalog OpenAction reaches the configured runner: {controls:?}"
    );
    assert!(
        controls
            .iter()
            .any(|seen| seen.script == "/* existing global control */ void 0;"),
        "positive control: existing name-tree library script also runs: {controls:?}"
    );
    log.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .calls
        .clear();

    let asked: Vec<Event> = viewer
        .handle(Command::Activate(ObjectId::new(5, 0)))
        .collect();
    assert!(
        asked.iter().any(|event| matches!(event, Event::NeedsFile {
            purpose: Purpose::ImportData,
            name,
            ..
        } if name == "answer.fdf")),
        "the actual ImportData action requests the FDF: {asked:?}"
    );
    let supplied: Vec<Event> = viewer
        .handle(Command::Supply {
            purpose: Purpose::ImportData,
            bytes: Some(fdf()),
        })
        .collect();
    let import_notes = notes(&supplied);
    assert!(
        import_notes
            .iter()
            .any(|note| note.contains("1 field(s) from answer.fdf, into 1 widget(s)")),
        "positive control: the supplied FDF value reached its matching target widget: {import_notes:?}"
    );

    let calls = log
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .calls
        .clone();
    let position = |script: &str| calls.iter().position(|seen| seen.script == script);
    let Some(doc) = position("/* Doc function */ function fdfHelper(){return 1;}") else {
        panic!("Table 248 /Doc script should run: {calls:?}");
    };
    let Some(before) = position("/* before import */ void 0;") else {
        panic!("Table 248 /Before script should run: {calls:?}");
    };
    let Some(after) = position("/* after import */ void 0;") else {
        panic!("Table 248 /After script should run: {calls:?}");
    };
    let Some(rights_ready) = position("/* after permissions ready */ void 0;") else {
        panic!("Table 248 /AfterPermsReady script should run: {calls:?}");
    };
    assert!(doc < before, "Doc scripts execute before Before: {calls:?}");
    assert_eq!(value_for(&calls[before], "name"), Some("old"));
    assert_eq!(value_for(&calls[after], "name"), Some("imported"));
    assert_eq!(value_for(&calls[rights_ready], "name"), Some("imported"));
    assert!(
        import_notes
            .iter()
            .all(|note| !note.contains("/JavaScript: document-level scripts, excluded")),
        "with a runner supplied, supported in-document scripts should not be blanket-declined: {import_notes:?}"
    );
}
