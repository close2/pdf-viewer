//! Table 32 JavaScript library entries decode §7.9.3 text streams before dispatch.

#![expect(
    clippy::expect_used,
    reason = "a malformed fixture or unsupported route is a setup failure"
)]

use std::io::Write as _;
use std::sync::{Arc, Mutex};

use pdf_model::view::{ScriptEvent, ScriptResult, ScriptRunner, ViewState};
use pdf_syntax::{Document, Object, ObjectId};

const SCRIPT: &str = "// café\nvoid 0;";

#[derive(Debug, Default)]
struct Capture(Mutex<Vec<String>>);

impl ScriptRunner for Capture {
    fn run(&self, event: &ScriptEvent<'_>) -> ScriptResult {
        self.0
            .lock()
            .expect("capture lock")
            .push(event.script.to_owned());
        ScriptResult {
            rc: true,
            value: None,
            change: None,
            edits: Vec::new(),
            report: Vec::new(),
        }
    }
}

fn document(js_value: &[u8], is_stream: bool) -> Vec<u8> {
    let mut out = b"%PDF-2.0\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = vec![0_u64];
    let mut object = |number: u32, body: &[u8]| {
        offsets.push(out.len() as u64);
        out.extend_from_slice(format!("{number} 0 obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    };

    object(
        1,
        b"<< /Type /Catalog /Pages 2 0 R /Names << /JavaScript 4 0 R >> >>",
    );
    object(2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    object(
        3,
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Resources << >> >>",
    );
    object(4, b"<< /Names [(helpers) 5 0 R] >>");
    object(
        5,
        if is_stream {
            b"<< /S /JavaScript /JS 6 0 R >>"
        } else {
            // Direct UTF-8 text string; its marker and payload match the stream's decoded bytes.
            b"<< /S /JavaScript /JS <EFBBBF2F2F20636166C3A90A766F696420303B> >>"
        },
    );
    if is_stream {
        let mut body = format!(
            "<< /Length {} /Filter /FlateDecode >>\nstream\n",
            js_value.len()
        )
        .into_bytes();
        body.extend_from_slice(js_value);
        body.extend_from_slice(b"\nendstream");
        object(6, &body);
    }
    let xref = out.len();
    let size = offsets.len();
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<00112233445566778899AABBCCDDEEFF> <00112233445566778899AABBCCDDEEFF>] >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out
}

fn run(bytes: Vec<u8>) -> Vec<String> {
    let document = Document::open(bytes).expect("the PDF fixture parses");
    assert!(
        !document.was_recovered(),
        "the fixture has exact xref offsets"
    );
    if let Object::Stream(stream) = document.get(ObjectId::new(6, 0)) {
        let decoded = document
            .decoded_stream_data(&stream)
            .expect("FlateDecode succeeds");
        assert_eq!(
            decoded.as_ref(),
            [b"\xEF\xBB\xBF".as_slice(), SCRIPT.as_bytes()]
                .concat()
                .as_slice(),
            "the unencoded stream bytes carry the same UTF-8 text-string marker and text"
        );
    }
    let capture = Arc::new(Capture::default());
    let mut view = ViewState::of(&document);
    view.run_scripts_with(Some(Arc::clone(&capture) as Arc<dyn ScriptRunner>));
    assert_eq!(view.run_open_scripts(&document, 0), 1);
    assert!(
        view.script_reports().is_empty(),
        "{:#?}",
        view.script_reports()
    );
    drop(view);
    Arc::try_unwrap(capture)
        .expect("the runner was dropped after the open sequence")
        .0
        .into_inner()
        .expect("captured script")
}

#[test]
fn table32_javascript_text_stream_decodes_like_the_equivalent_text_string() {
    let utf8 = [b"\xEF\xBB\xBF".as_slice(), SCRIPT.as_bytes()].concat();
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder
        .write_all(&utf8)
        .expect("compresses the text stream");
    let compressed = encoder.finish().expect("finishes the text stream");

    assert_eq!(
        run(document(&compressed, true)),
        vec![SCRIPT.to_owned()],
        "the Table 32 library path hands the text decoded from the stream to its runner"
    );
    assert_eq!(
        run(document(&[], false)),
        vec![SCRIPT.to_owned()],
        "the equivalent text-string form is a positive control"
    );
}
