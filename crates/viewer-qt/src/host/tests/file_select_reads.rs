//! ISO 32000-2 §12.7.5.3, Table 231 bit 21: a file-select control's file is read once, when the
//! person finishes with the field, never per keystroke (ADR 1216, ADR 3041). Drives the Qt host's
//! Rust callbacks without a `QApplication`, as `textEdited` and `editingFinished` call them.

use std::io::Write as _;
use std::path::PathBuf;
use std::time::Instant;

use super::Host;
use pdf_model::view::WidgetAppearances;
use viewer_core::{Answer, Query};
use viewer_host::{ReaderWords, Scripts, Settings, trace::Trace};

/// A one-page form whose one field is a single-line text field with Table 231 bit 21 set.
fn file_select_form() -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] \
         /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv 5 0 R >> >> >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [4 0 R] >>",
        "<< /Type /Annot /Subtype /Widget /P 3 0 R /FT /Tx /T (upload) /Ff 1048576 \
         /Rect [20 300 220 330] /DA (/Helv 12 Tf 0 g) >>",
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
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

fn temporary(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("pdf-viewer-qt-{name}-{}", std::process::id()))
}

#[test]
fn a_typed_path_is_read_once_when_the_field_is_committed() {
    let document = temporary("file-select.pdf");
    std::fs::write(&document, file_select_form()).expect("write the form");
    let chosen = temporary("file-select-chosen.txt");
    std::fs::write(&chosen, b"typed").expect("write the chosen file");
    let path = chosen.to_string_lossy().into_owned();

    let mut host = Host::open(
        &document,
        None,
        WidgetAppearances::Delegated,
        Settings {
            restrictions: viewer_core::RestrictionPolicy::uniform(
                viewer_core::RestrictionLevel::Off,
            ),
            links: viewer_host::Links::Refuse,
            remote_documents: viewer_host::RemoteDocuments::Refuse,
            embedded_documents: viewer_host::EmbeddedDocuments::Refuse,
            submissions: viewer_host::Submissions::Refuse,
            separations: false,
            scripts: Scripts::Off,
            launched_documents: viewer_host::LaunchedDocuments::Refuse,
            media_files: viewer_host::MediaFiles::Refuse,
        },
        ReaderWords::default(),
        Trace::off(Instant::now()),
    )
    .expect("the form opens in the Qt host");
    host.resized(800, 1000, 1.0);
    super::drawn(&mut host);
    let index = host
        .controls()
        .iter()
        .position(|control| control.field == "upload")
        .expect("the file-select field has a placed Qt control");

    let before = viewer_host::chosen_reads();
    for end in 1..=path.len() {
        if let Some(prefix) = path.get(..end) {
            host.set_control(index, prefix);
        }
    }
    assert_eq!(
        viewer_host::chosen_reads(),
        before,
        "no keystroke reads a file"
    );
    host.commit_control(index, true);
    // The keyboard leaving after Enter is `editingFinished` again, and finds nothing typed.
    host.commit_control(index, false);
    assert_eq!(
        viewer_host::chosen_reads(),
        before.saturating_add(1),
        "one read for the typed path and its commit"
    );

    let Answer::Fields(fields) = host.state.viewer.query(Query::Fields) else {
        panic!("the host answers its fields");
    };
    let value = fields
        .into_iter()
        .find(|field| field.name.qualified == "upload")
        .and_then(|field| field.value)
        .map(|value| value.text);
    let _ = std::fs::remove_file(&document);
    let _ = std::fs::remove_file(&chosen);
    assert_eq!(
        value.as_deref(),
        Some(path.as_str()),
        "the path is the field's text"
    );
}
