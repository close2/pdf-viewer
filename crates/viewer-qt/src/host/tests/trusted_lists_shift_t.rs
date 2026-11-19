//! Shift + T on a signed document whose policy references a trusted list: the reference reaches
//! the question and the refresh (ADR 2936, ETSI TS 119 172-2 clause 4.22.2.1 b)). Drives the Qt
//! host's Rust callbacks without a `QApplication`, as `keyPressEvent` and the question dialogue
//! call them.

use std::path::PathBuf;
use std::time::Instant;

use super::super::Host;
use pdf_model::view::WidgetAppearances;
use viewer_host::{ReaderWords, Scripts, Settings, trace::Trace};

#[path = "../../../../../tools/conformance/test-support/signed_under_a_policy.rs"]
mod signed_under_a_policy;

/// `Qt::Key_T`.
const T: u32 = 0x54;

fn temporary(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("pdf-viewer-qt-{name}-{}", std::process::id()))
}

/// The references the question Shift + T put holds, and its text; `None` where it put none.
fn shift_t(host: &mut Host) -> Option<(Vec<String>, String)> {
    host.key(T, true, false);
    match &host.question {
        Some((viewer_host::native_window::Asked::TrustedLists { references }, text)) => {
            Some((references.clone(), text.clone()))
        }
        _ => None,
    }
}

/// **With the policy's copy bound, the question at `ask` names the list the policy references and
/// holds it for the refresh, and the yes starts the refresh.** The calibration is the same document
/// before the copy is bound, whose question names none: only a bound policy is anybody's
/// agreement.
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
        ReaderWords {
            trusted_lists: Some(directory.clone()),
            ..ReaderWords::default()
        },
        Trace::off(Instant::now()),
    )
    .expect("the signed document opens in the Qt host");
    host.resized(400, 400, 1.0);
    super::drawn(&mut host);
    // `ask` after the open, as the menu would set it: at the open it would first put the question
    // about fetching the policy itself, and Shift + T is declined while that one waits.
    host.state.restrictions = host
        .state
        .restrictions
        .with(viewer_host::ActLevel::Submissions(
            viewer_host::Submissions::Ask,
        ));

    let unbound = shift_t(&mut host).expect("ask puts the question");
    assert_eq!(unbound.0, Vec::<String>::new(), "{}", unbound.1);
    assert!(
        !unbound.1.contains(signed_under_a_policy::REFERENCE),
        "an unbound policy is nobody's agreement: {}",
        unbound.1
    );
    host.answer_question(false);

    let focused = host.state.documents.focused();
    assert_eq!(
        signed_under_a_policy::bind(&mut host.state.viewer, focused),
        1,
        "the signature publishes its policy"
    );
    let (references, text) = shift_t(&mut host).expect("ask puts the question");
    assert_eq!(references, [signed_under_a_policy::REFERENCE], "{text}");
    assert!(
        text.contains(signed_under_a_policy::REFERENCE),
        "the question names the referenced list: {text}"
    );
    assert!(host.state.refresher.interval().is_none(), "nothing yet");
    host.answer_question(true);
    assert!(
        host.state.refresher.interval().is_some(),
        "the yes starts the refresh"
    );
    let _ = std::fs::remove_file(&document);
    let _ = std::fs::remove_dir_all(&directory);
}
