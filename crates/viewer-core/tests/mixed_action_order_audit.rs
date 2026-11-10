//! Public viewer regression for `/Next` ordering across non-script and JavaScript actions.

#![expect(
    clippy::expect_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    reason = "the test fixture and observable endpoint must fail loudly if their stated objects are absent"
)]

use std::sync::{Arc, Mutex};

use pdf_model::{
    action::Trigger,
    view::{ScriptEdit, ScriptEvent, ScriptResult, ScriptRunner, ScriptSite},
};
use pdf_syntax::Document;
use viewer_core::{
    Answer, Command, DocumentId, PointerAction, Query, ScriptRunners, Scripting, Viewer,
};

const DOCUMENT: DocumentId = DocumentId(1);

type Seen = Arc<Mutex<Vec<(ScriptSite, bool, String)>>>;

#[derive(Debug)]
struct RecordingRunner(Seen);

impl ScriptRunner for RecordingRunner {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        let layer_on = event
            .document
            .and_then(|document| document.layers.iter().find(|layer| layer.name == "switch"))
            .map(|layer| layer.on)
            .expect("the runner receives the document's optional-content state");
        self.0
            .lock()
            .expect("the test's record is not poisoned")
            .push((event.site, layer_on, event.script.to_owned()));
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
struct RecordingRunners(Seen);

impl ScriptRunners for RecordingRunners {
    fn runner(&self) -> Arc<dyn ScriptRunner> {
        Arc::new(RecordingRunner(Arc::clone(&self.0)))
    }
}

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

/// `state_first` selects the order of the root action and its `/Next` script.
fn mixed_chain_pdf(state_first: bool) -> Vec<u8> {
    let (action6, action7) = if state_first {
        (
            b"<< /S /SetOCGState /State [/ON 5 0 R] /Next 7 0 R >>".to_vec(),
            b"<< /S /JavaScript /JS (void 0;) >>".to_vec(),
        )
    } else {
        (
            b"<< /S /JavaScript /JS (void 0;) /Next 7 0 R >>".to_vec(),
            b"<< /S /SetOCGState /State [/ON 5 0 R] >>".to_vec(),
        )
    };
    assemble(&[
        b"<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [5 0 R] /D << /Order [5 0 R] /OFF [5 0 R] >> >> >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Annots [4 0 R] >>".to_vec(),
        b"<< /Type /Annot /Subtype /Link /P 3 0 R /Rect [10 10 30 30] /A 6 0 R >>".to_vec(),
        b"<< /Type /OCG /Name (switch) >>".to_vec(),
        action6,
        action7,
    ])
}

fn open(state_first: bool, seen: Seen) -> Viewer {
    let bytes = mixed_chain_pdf(state_first);
    let parsed = Document::open(bytes.clone()).expect("complete valid PDF fixture");
    assert!(!parsed.was_recovered(), "fixture has exact xref offsets");
    let mut viewer = Viewer::new(200, 200, 1.0);
    viewer
        .handle(Command::Scripts(Scripting::Run(Arc::new(
            RecordingRunners(seen),
        ))))
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
    viewer
}

fn layer_on(viewer: &Viewer) -> bool {
    let Answer::Layers(layers) = viewer.query(Query::Layers) else {
        panic!("the catalog states an optional-content order");
    };
    layers
        .iter()
        .find_map(|layer| match layer {
            viewer_core::Layer::Group {
                name: Some(name),
                on,
                ..
            } if name == "switch" => Some(*on),
            _ => None,
        })
        .expect("the /OCGs array contains the switch group")
}

fn activate_link(viewer: &mut Viewer) {
    let Answer::Geometry(geometry) = viewer.query(Query::PageGeometry(0)) else {
        panic!("the one page is interpreted and placed");
    };
    let at = (
        geometry.origin.0 + 20.0 * geometry.scale,
        geometry.origin.1 + 80.0 * geometry.scale,
    );
    assert!(matches!(
        viewer.query(Query::LinkAt(at)),
        Answer::Link(true)
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
fn a_next_script_observes_preceding_non_script_action_state() {
    // Positive order control: when JavaScript precedes SetOCGState in /Next order, the runner
    // sees OFF and the action subsequently turns the group ON.
    let control_seen: Seen = Arc::default();
    let mut control = open(false, Arc::clone(&control_seen));
    assert!(!layer_on(&control));
    activate_link(&mut control);
    assert!(layer_on(&control), "the later SetOCGState action ran");
    assert_eq!(
        *control_seen.lock().expect("control record"),
        [(
            ScriptSite::Annotation(Trigger::Up),
            false,
            "void 0;".to_owned()
        )]
    );

    // In the regression order, SetOCGState is the root and the script is its /Next. Table 196
    // requires the script to run after the state change, so its public ScriptEvent snapshot must
    // show ON. The final Query independently confirms the action changed viewer state.
    let ordered_seen: Seen = Arc::default();
    let mut ordered = open(true, Arc::clone(&ordered_seen));
    assert!(!layer_on(&ordered), "the control configuration starts OFF");
    activate_link(&mut ordered);
    assert!(layer_on(&ordered), "the root SetOCGState action ran");
    assert_eq!(
        *ordered_seen.lock().expect("ordered record"),
        [(
            ScriptSite::Annotation(Trigger::Up),
            true,
            "void 0;".to_owned()
        )],
        "the /Next script observes the root action's already-applied layer state"
    );
}
