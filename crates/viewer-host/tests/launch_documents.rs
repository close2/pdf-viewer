//! ISO 32000-2 §12.6.4.6's launch action, where Table 207's `/F` names a file this reader may open.
//!
//! Table 207 makes `/F` "[t]he application that shall be launched or the document that shall be
//! opened or printed", and `/NewWindow` "shall be ignored if the file designated by the F entry is
//! not a PDF document" — so the table expects a launch action to name a PDF, and opening one is the
//! act the reader's remote-documents level already decides for Tables 203, 209 and 253 (ADRs 1368,
//! 1358). What only a host can do is the half checked here: the level, and the question's words.

/// ISO 32000-2 §12.6.4.6's file is asked about under the four levels, and the question says both
/// things it may turn out to be (ADRs 1368, 1358).
///
/// Table 207 makes `/F` "[t]he application that shall be launched or the document that shall be
/// opened or printed", so a person asked whether to let it be opened is told what happens to each:
/// a PDF opens — beside, where `/NewWindow` asks — and anything else starts nothing.
#[test]
fn a_launch_actions_file_is_a_remote_document_and_the_question_says_what_it_may_be() {
    use std::path::Path;
    use viewer_core::Purpose;
    use viewer_host::policy::{asked_for, under_remote_documents};
    use viewer_host::{Remote, RemoteDocuments, remote};

    assert!(
        under_remote_documents(Purpose::LaunchDocument),
        "the reader's remote-documents level decides it, as it decides Table 203's"
    );
    assert_eq!(asked_for(Purpose::LaunchDocument), "Launch");
    let asked = |beside| match remote(
        Some(Path::new("/documents")),
        "file1.pdf",
        RemoteDocuments::Ask,
        Purpose::LaunchDocument,
        beside,
    ) {
        Remote::Ask { question, .. } => question.reasons,
        other => panic!("ask puts the question: {other:?}"),
    };
    let beside = asked(true);
    assert!(
        beside.contains("beside the one you are reading")
            && beside.contains("if it is a PDF")
            && beside.contains("starts none")
            && beside.contains("§12.6.4.6"),
        "{beside}"
    );
    assert!(
        asked(false).contains("in place of the one you are reading"),
        "{}",
        asked(false)
    );
    // And the refusal level refuses it by the launch's own name.
    assert!(matches!(
        remote(
            Some(Path::new("/documents")),
            "file1.pdf",
            RemoteDocuments::Refuse,
            Purpose::LaunchDocument,
            true,
        ),
        Remote::Refuse(said) if said.starts_with("Launch: declined")
    ));
}
