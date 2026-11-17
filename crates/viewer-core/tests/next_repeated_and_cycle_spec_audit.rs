//! Public-viewer probes for §12.6.2 repeated references and cycle termination.

#![expect(clippy::expect_used, clippy::panic, clippy::arithmetic_side_effects)]

use std::sync::{Arc, Mutex};

use pdf_model::view::{ScriptEdit, ScriptEvent, ScriptResult, ScriptRunner};
use viewer_core::{Command, DocumentId, PointerAction, Query, ScriptRunners, Scripting, Viewer};

type Seen = Arc<Mutex<Vec<String>>>;

#[derive(Debug)]
struct Runner(Seen);

impl ScriptRunner for Runner {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        self.0
            .lock()
            .expect("record is not poisoned")
            .push(event.script.to_owned());
        ScriptResult {
            rc: true,
            value: None,
            change: None,
            edits: Vec::<ScriptEdit>::new(),
            report: Vec::new(),
        }
    }
}

#[derive(Debug)]
struct Runners(Seen);

impl ScriptRunners for Runners {
    fn runner(&self) -> Arc<dyn ScriptRunner> {
        Arc::new(Runner(Arc::clone(&self.0)))
    }
}

fn assemble(objects: &[&str]) -> Vec<u8> {
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", index + 1).as_bytes());
    }
    let xref = bytes.len();
    bytes.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
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

fn pdf(chain: &str, tail: &[&str]) -> Vec<u8> {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Annots [4 0 R] >>",
        "<< /Type /Annot /Subtype /Link /Rect [10 10 30 30] /A 6 0 R >>",
        "<< /Length 0 >>\nstream\n\nendstream",
        chain,
    ];
    objects.extend_from_slice(tail);
    assemble(&objects)
}

fn activate(bytes: Vec<u8>, seen: Seen) {
    let parsed = pdf_syntax::Document::open(bytes.clone()).expect("valid fixture");
    assert!(!parsed.was_recovered(), "fixture's xref is exact");
    let mut viewer = Viewer::new(200, 200, 1.0);
    viewer
        .handle(Command::Scripts(Scripting::Run(Arc::new(Runners(seen)))))
        .for_each(drop);
    viewer
        .handle(Command::Open {
            id: DocumentId(1),
            bytes: bytes.into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);
    viewer.handle(Command::Presented).for_each(drop);
    let viewer_core::Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("one page has geometry");
    };
    // Default user-space point (20, 20), transformed to device coordinates.
    let at = (
        geometry.origin.0 + 20.0 * geometry.scale,
        geometry.origin.1 + 80.0 * geometry.scale,
    );
    assert!(matches!(
        viewer.query(Query::LinkAt(at)),
        viewer_core::Answer::Link(true)
    ));
    viewer
        .handle(Command::Pointer {
            at,
            action: PointerAction::Pressed,
        })
        .for_each(drop);
    viewer
        .handle(Command::Pointer {
            at,
            action: PointerAction::Released,
        })
        .for_each(drop);
}

#[test]
fn next_array_repeats_an_indirect_action_but_a_path_cycle_stops() {
    let repeated: Seen = Arc::default();
    // Object 7 is the same indirect action in both array positions.
    activate(
        pdf(
            "<< /S /JavaScript /JS ('root';) /Next [7 0 R 7 0 R] >>",
            &["<< /S /JavaScript /JS ('leaf';) >>"],
        ),
        Arc::clone(&repeated),
    );
    assert_eq!(
        *repeated.lock().expect("record"),
        [
            "'root';".to_owned(),
            "'leaf';".to_owned(),
            "'leaf';".to_owned()
        ],
        "the two array occurrences are two performances, in order"
    );

    let control: Seen = Arc::default();
    // Independent equivalent control: two different objects with the same action.
    activate(
        pdf(
            "<< /S /JavaScript /JS ('root';) /Next [7 0 R 8 0 R] >>",
            &[
                "<< /S /JavaScript /JS ('leaf';) >>",
                "<< /S /JavaScript /JS ('leaf';) >>",
            ],
        ),
        Arc::clone(&control),
    );
    assert_eq!(
        *control.lock().expect("record"),
        [
            "'root';".to_owned(),
            "'leaf';".to_owned(),
            "'leaf';".to_owned()
        ]
    );

    let cycle: Seen = Arc::default();
    activate(
        pdf(
            "<< /S /JavaScript /JS ('root';) /Next 7 0 R >>",
            &["<< /S /JavaScript /JS ('leaf';) /Next 6 0 R >>"],
        ),
        Arc::clone(&cycle),
    );
    assert_eq!(
        *cycle.lock().expect("record"),
        ["'root';".to_owned(), "'leaf';".to_owned()],
        "a self-referential path terminates without replaying the root"
    );
}
