// Draft destination: crates/viewer-qt/src/host/tests/choice_commit_audit.rs
// Add `mod choice_commit_audit;` beside `mod ef_host_route;` in host/tests.rs.
// This drives Qt Host's Rust callback without QApplication; it does not exercise the C++ signal.

use std::io::Write as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use super::Host;
use pdf_model::view::WidgetAppearances;
use pdf_syntax::Document;
use viewer_core::{Answer, Command, FocusMove, Query};
use viewer_host::{ReaderWords, Scripts, Settings, trace::Trace};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct PdfPath(PathBuf);

impl PdfPath {
    fn new(bytes: &[u8]) -> Self {
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "pdf-viewer-qt-choice-commit-{}-{serial}.pdf",
            std::process::id()
        ));
        std::fs::write(&path, bytes).expect("write a complete PDF for the real host");
        Self(path)
    }
}

impl Drop for PdfPath {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn pdf(commit_on_selection: bool, text_control: bool) -> Vec<u8> {
    let field_type = if text_control {
        "/FT /Tx"
    } else {
        "/FT /Ch /Opt [(5) (7) (1)]"
    };
    let flags = if commit_on_selection { 1_i64 << 26 } else { 0 };
    let validate = "/AA << /V << /S /JavaScript /JS \
                    (AFRange_Validate\\(true, 5, true, 10\\)) >> >>";
    let content = "q Q";
    let stream = |text: &str| {
        format!(
            "<< /Length {} >>\nstream\n{text}\nendstream",
            text.len().saturating_add(1)
        )
    };
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] \
         /NeedAppearances true /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv 5 0 R >> >> >> >>"
            .to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] \
         /Resources << >> /Contents 6 0 R /Annots [4 0 R] >>"
            .to_owned(),
        format!(
            "<< /Type /Annot /Subtype /Widget /P 3 0 R {field_type} /T (Choice) /Ff {flags} \
             /V (5) /Rect [20 300 220 330] /DA (/Helv 12 Tf 0 g) \
             {validate} >>"
        ),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"
            .to_owned(),
        stream(content),
    ];
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        write!(
            bytes,
            "{} 0 obj\n{object}\nendobj\n",
            index.saturating_add(1)
        )
        .expect("write into bytes");
    }
    let xref = bytes.len();
    let size = objects.len().saturating_add(1);
    write!(bytes, "xref\n0 {size}\n0000000000 65535 f \n").expect("write xref header");
    for offset in offsets {
        writeln!(bytes, "{offset:010} 00000 n ").expect("write xref entry");
    }
    write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    )
    .expect("write trailer");
    bytes
}

fn open_host(bytes: &[u8]) -> (PdfPath, Host) {
    let parsed = Document::open(bytes.to_vec()).expect("complete AcroForm PDF syntax");
    assert!(!parsed.was_recovered(), "fixture xref offsets are exact");
    let path = PdfPath::new(bytes);
    let host = Host::open(
        &path.0,
        None,
        WidgetAppearances::Delegated,
        Settings {
            restrictions: viewer_core::RestrictionPolicy::uniform(
                viewer_core::RestrictionLevel::On,
            ),
            links: viewer_host::Links::Refuse,
            remote_documents: viewer_host::RemoteDocuments::Refuse,
            embedded_documents: viewer_host::EmbeddedDocuments::Refuse,
            submissions: viewer_host::Submissions::Refuse,
            separations: false,
            scripts: Scripts::On,
            launched_documents: viewer_host::LaunchedDocuments::Refuse,
        },
        ReaderWords::default(),
        Trace::off(Instant::now()),
    )
    .expect("the complete AcroForm opens in the Qt host");
    (path, host)
}

fn field_value(host: &Host) -> String {
    let Answer::Fields(fields) = host.state.viewer.query(Query::Fields) else {
        panic!("the host viewer answers its AcroForm fields");
    };
    let field = fields
        .into_iter()
        .find(|field| field.name.qualified == "Choice")
        .expect("the fixture field is present");
    if let Some(value) = field.value {
        return value.text;
    }
    let pdf_model::form::Control::Choice(choice) = field.control else {
        panic!("the field is a choice control");
    };
    assert_eq!(choice.selected.len(), 1, "one selected item");
    choice.options[choice.selected[0]].label.clone()
}

fn select_invalid_choice(host: &mut Host) -> usize {
    let index = host
        .controls()
        .iter()
        .position(|control| control.field == "Choice")
        .expect("the choice field has a placed Qt control");
    assert_eq!(host.control_options(index), ["5", "7", "1"]);
    host.state
        .viewer
        .handle(Command::Focused(FocusMove::Next))
        .for_each(drop);
    assert_eq!(host.focused_control(), i32::try_from(index).unwrap_or(-1));
    host.choose_control(index, &[2]);
    index
}

fn prepared(commit_on_selection: bool, text_control: bool) -> (PdfPath, Host) {
    let (path, mut host) = open_host(&pdf(commit_on_selection, text_control));
    host.resized(800, 1000, 1.0);
    super::drawn(&mut host);
    assert_eq!(field_value(&host), "5");
    (path, host)
}

fn text_validator_control() {
    let (_path, mut host) = prepared(false, true);
    let index = host
        .controls()
        .iter()
        .position(|control| control.field == "Choice")
        .expect("the text control is placed");
    host.set_control(index, "1");
    assert_eq!(field_value(&host), "1", "invalid text is staged");
    host.commit_control(index, false);
    assert_eq!(
        field_value(&host),
        "5",
        "the same validator rejects text at commit"
    );
}

#[test]
fn text_commit_is_a_positive_control_for_the_acroform_validator() {
    text_validator_control();
}

#[test]
fn an_explicit_choice_commit_runs_the_acroform_validate_action() {
    text_validator_control();
    let (_path, mut host) = prepared(false, false);
    let index = select_invalid_choice(&mut host);
    assert_eq!(
        field_value(&host),
        "1",
        "clear-bit selection reaches the field"
    );
    host.commit_control(index, false);
    assert_eq!(
        field_value(&host),
        "5",
        "explicit choice commit must run /V and refuse 1"
    );
}

#[test]
fn commit_on_selection_runs_the_acroform_validate_action_in_the_qt_choice_callback() {
    text_validator_control();
    let (_clear_path, mut clear) = prepared(false, false);
    select_invalid_choice(&mut clear);
    assert_eq!(field_value(&clear), "1", "clear bit postpones validation");
    let (_set_path, mut set) = prepared(true, false);
    select_invalid_choice(&mut set);
    assert_eq!(
        field_value(&set),
        "5",
        "set bit must validate the choice immediately"
    );
}
