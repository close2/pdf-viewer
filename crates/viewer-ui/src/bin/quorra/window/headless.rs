//! This window driven without a display: an [`App`] made by [`App::new`], the launch path's own
//! constructor, its document opened by the launch path's own `open_document` on the thread the
//! launch path uses and joined as `resumed` joins it, and every key and press handed to the
//! handlers winit's events reach. Nothing is mocked; what is absent is the window, which the
//! handlers already treat as not yet made (`App::redraw`, `App::window_now`).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use viewer_core::{Answer, Command, Query, RestrictionLevel, RestrictionPolicy};
use winit::event::ElementState;
use winit::keyboard::{Key, ModifiersState, NamedKey};

use crate::app::{App, Launching, Pending};

#[path = "../../../../../../tools/conformance/test-support/signed_under_a_policy.rs"]
mod signed_under_a_policy;

/// A file under the system's temporary directory, named for this process and `name`.
fn temporary(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("quorra-headless-{}-{name}", std::process::id()))
}

/// A one-revision file of `objects`, numbered from 1, with its exact cross-reference table.
fn file_of(objects: &[&str]) -> Vec<u8> {
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{object}\nendobj\n", index.saturating_add(1));
    }
    let xref = out.len();
    let size = objects.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

/// The window on the document at `path`, with `reader`'s words, made as `main` makes it and given
/// an 800 by 1000 viewport, as `resumed` gives it the window's size.
fn window_on(path: &Path, reader: viewer_host::ReaderWords) -> App {
    let trusting = reader.trusting();
    let opening = viewer_host::Loading::start({
        let path = path.to_path_buf();
        move || {
            crate::open_document(
                &path,
                None,
                None,
                crate::Policies {
                    restrictions: RestrictionPolicy::uniform(RestrictionLevel::Off),
                    reader,
                    separations: false,
                },
            )
        }
    });
    let launch = crate::timing::Launch::new();
    let trace = crate::trace::Trace::off(launch.began);
    let mut app = App::new(Launching {
        path: path.to_path_buf(),
        also: Vec::new(),
        fragment: None,
        trace,
        processor: true,
        proxy_pages: 0,
        supersample: 1,
        coverage: crate::arguments::CoverageChoice::Auto,
        backend: None,
        backend_asked_for: false,
        standing: viewer_host::Restrictions::new(RestrictionPolicy::uniform(RestrictionLevel::Off)),
        links: viewer_host::Links::Refuse,
        remote_documents: viewer_host::RemoteDocuments::Refuse,
        archive: None,
        trusting,
        separations: false,
        chrome: None,
        opening: Some(opening),
        ahead: None,
        instancing: None,
        launch,
    });
    app.take_the_opening();
    app.dispatch(Command::Resize {
        width: 800,
        height: 1000,
        scale: 1.0,
    });
    app
}

/// The core's point for page point `(x, y)` on page one, as the window's pointer arrives there.
fn at(app: &App, x: f32, y: f32) -> (f32, f32) {
    let Answer::Geometry(geometry) = app.viewer.query(Query::PageGeometry(0)) else {
        panic!("page one has a geometry once the window has a size");
    };
    (
        geometry.origin.0 + x * geometry.scale,
        geometry.origin.1 + (geometry.page.height - y) * geometry.scale,
    )
}

/// ISO 32000-2 §12.7.5.3, Table 231 bit 21: a file-select control's file is read once, at the
/// Enter that commits the typed path, never per keystroke (ADR 1216, ADR 3041) — counted here at
/// the window, through the press that aims the keyboard and the keys that type and commit.
#[test]
fn a_typed_path_is_read_once_at_enter() {
    let document = temporary("file-select.pdf");
    std::fs::write(
        &document,
        file_of(&[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] \
             /DA (/Helv 12 Tf 0 g) /DR << /Font << /Helv 5 0 R >> >> >> >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 400 400] /Annots [4 0 R] >>",
            "<< /Type /Annot /Subtype /Widget /P 3 0 R /FT /Tx /T (upload) /Ff 1048576 \
             /Rect [20 300 220 330] /DA (/Helv 12 Tf 0 g) >>",
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
        ]),
    )
    .expect("write the form");
    let chosen = temporary("file-select-chosen.txt");
    std::fs::write(&chosen, b"typed").expect("write the chosen file");
    let path = chosen.to_string_lossy().into_owned();

    let mut app = window_on(&document, viewer_host::ReaderWords::default());
    let field = at(&app, 120.0, 315.0);
    app.click_page(field, ElementState::Pressed);
    app.click_page(field, ElementState::Released);
    assert!(
        app.typing.is_some(),
        "the press gives the field the keyboard"
    );

    let before = viewer_host::chosen_reads();
    for character in path.chars() {
        let mut typed = [0; 4];
        app.pressed(&Key::Character(character.encode_utf8(&mut typed)));
    }
    assert_eq!(
        viewer_host::chosen_reads(),
        before,
        "no keystroke reads a file"
    );
    app.pressed(&Key::Named(NamedKey::Enter));
    assert_eq!(
        viewer_host::chosen_reads(),
        before.saturating_add(1),
        "one read for the typed path and its Enter"
    );
    assert!(app.typing.is_none(), "Enter gives the keyboard back");

    let Answer::Fields(fields) = app.viewer.query(Query::Fields) else {
        panic!("the window's core answers its fields");
    };
    let value = fields
        .into_iter()
        .find(|one| one.name.qualified == "upload")
        .and_then(|one| one.value)
        .map(|value| value.text);
    let _ = std::fs::remove_file(&document);
    let _ = std::fs::remove_file(&chosen);
    assert_eq!(
        value.as_deref(),
        Some(path.as_str()),
        "the path is the field's text"
    );
}

/// What Shift + T leaves waiting for the person's answer: the references it will hand the
/// refresh, or `None` where it put no question.
fn shift_t(app: &mut App) -> Option<Vec<String>> {
    app.modifiers_changed(ModifiersState::SHIFT);
    app.pressed(&Key::Character("T"));
    app.modifiers_changed(ModifiersState::empty());
    match &app.asked {
        Some(Pending::TrustedLists { references }) => Some(references.clone()),
        _ => None,
    }
}

/// **A signed document's policy reference reaches the refresh through Shift + T** (ADR 2936,
/// ETSI TS 119 172-2 clause 4.22.2.1 b)): with the policy's copy bound, the question at `ask`
/// holds the list the policy references, which is what its yes hands the refresh, and the yes
/// starts it. The calibration is the same document before the copy is bound, whose question holds
/// none: only a bound policy is anybody's agreement.
#[test]
fn a_bound_policy_s_reference_goes_with_shift_t_to_the_refresh() {
    let directory = temporary("trusted-lists");
    // A directory left by an earlier run is the one thing to clear; its absence is the usual case.
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a temporary directory");
    std::fs::write(directory.join("OJEU-LOTL-Certs-Set.der"), [0x30, 0x00])
        .expect("the certificate file");
    std::fs::write(
        directory.join(pdf_signature::trusted_list::authentication::OJEU_LOTL_LOC),
        "http://127.0.0.1:9/never-answered.xml",
    )
    .expect("the location file");
    let document = temporary("signed.pdf");
    std::fs::write(&document, signed_under_a_policy::document()).expect("write the document");

    let mut app = window_on(
        &document,
        viewer_host::ReaderWords {
            trusted_lists: Some(directory.clone()),
            ..viewer_host::ReaderWords::default()
        },
    );
    app.restrictions = app.restrictions.with(viewer_host::ActLevel::Submissions(
        viewer_host::Submissions::Ask,
    ));

    assert_eq!(
        shift_t(&mut app),
        Some(Vec::new()),
        "an unbound policy is nobody's agreement"
    );
    app.pressed(&Key::Named(NamedKey::Escape));
    assert!(app.asked.is_none(), "Escape answers the question");

    let focused = app.documents.focused();
    assert_eq!(
        signed_under_a_policy::bind(&mut app.viewer, focused),
        1,
        "the signature publishes its policy"
    );
    assert_eq!(
        shift_t(&mut app),
        Some(vec![signed_under_a_policy::REFERENCE.to_owned()]),
        "the question holds the referenced list"
    );
    assert!(app.refresher.interval().is_none(), "nothing fetched yet");
    app.pressed(&Key::Named(NamedKey::Enter));
    assert!(
        app.refresher.interval().is_some(),
        "the yes starts the refresh"
    );
    let _ = std::fs::remove_file(&document);
    let _ = std::fs::remove_dir_all(&directory);
}
