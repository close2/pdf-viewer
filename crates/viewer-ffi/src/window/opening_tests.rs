//! A `quorra_window` hands a clip, a launched document and a link to the program hosting it,
//! which opens them with the platform's own opener — the `WinUI` window's
//! `Windows.System.Launcher` (ADRs 3217, 3280).
//!
//! What these hold is the library's half: the level decides as in every window, the clip is
//! written into its private directory, and the hand-off is a `QUORRA_OWED_OPEN_FILE` or
//! `QUORRA_OWED_OPEN_URI` record rather than a call to the desktop portal; a failure the host
//! reports through `quorra_window_open_failed` is said. The C# that carries the records out is
//! compiled by CI's Windows job alone.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use viewer_host::native_window::NativeWindow as _;
use viewer_host::{ActLevel, MediaFiles};

use super::{OwedKind, Window};

/// One empty page.
fn document(path: &Path) {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] /Resources << >> >>",
    ];
    let mut pdf = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        writeln!(
            pdf,
            "{} 0 obj\n{object}\nendobj",
            index.checked_add(1).expect("three fixture objects")
        )
        .expect("write");
    }
    let start = pdf.len();
    pdf.push_str("xref\n0 4\n0000000000 65535 f \n");
    for offset in offsets {
        writeln!(pdf, "{offset:010} 00000 n ").expect("write");
    }
    write!(
        pdf,
        "trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n"
    )
    .expect("write");
    std::fs::write(path, pdf).expect("fixture");
}

/// A window on one empty page at the media level `level`.
fn window(name: &str, level: MediaFiles) -> (Window, PathBuf) {
    let path =
        std::env::temp_dir().join(format!("quorra-opening-{}-{name}.pdf", std::process::id()));
    document(&path);
    let mut window =
        Window::open(&path, None, viewer_host::policy::Words::default()).expect("open");
    window.resize(400, 400, 1.0).expect("resize");
    window.state.restrictions = window.state.restrictions.with(ActLevel::MediaFiles(level));
    let _ = window.take_owed();
    (window, path)
}

/// Every record the window owes since it was last asked, as kind and text.
fn owed(window: &mut Window) -> Vec<(OwedKind, String)> {
    let owed = window.take_owed();
    (0..owed.len())
        .filter_map(|index| {
            let kind = owed.kind(index)?;
            Some((kind, owed.text(index).unwrap_or_default().to_owned()))
        })
        .collect()
}

/// The clip a document embeds: a RIFF WAVE header is enough to be no program.
const RIFF: &[u8] = b"RIFF\x24\x00\x00\x00WAVEfmt ";

fn embedded() -> viewer_core::Media {
    viewer_core::Media::Embedded {
        name: "intro.wav".to_owned(),
        bytes: RIFF.to_vec(),
    }
}

/// The written clip's private directory, removed: the test leaves nothing under the user's
/// runtime directory.
fn remove_written(path: &str) {
    if let Some(directory) = Path::new(path).parent() {
        std::fs::remove_dir_all(directory).expect("the clip's directory is removed");
    }
}

/// At `open`, an embedded clip is written as every window writes it and owed to the host by its
/// path, and the sentence names the host's opener rather than the desktop portal.
#[test]
fn an_embedded_clip_is_owed_to_the_host_by_its_path() {
    let (mut window, pdf) = window("embedded", MediaFiles::Open);
    let document = window.state.documents.focused();
    window.media(document, Some("audio/wav"), embedded(), None);
    let records = owed(&mut window);
    let opened: Vec<&String> = records
        .iter()
        .filter(|(kind, _)| *kind == OwedKind::OpenFile)
        .map(|(_, text)| text)
        .collect();
    assert_eq!(opened.len(), 1, "{records:?}");
    let written = opened[0];
    assert!(written.ends_with("clip.wav"), "{written}");
    assert_eq!(std::fs::read(written).expect("the clip is written"), RIFF);
    assert!(
        records
            .iter()
            .any(|(kind, text)| *kind == OwedKind::Said
                && text.contains(super::opening::HOST_OPENER)),
        "{records:?}"
    );
    remove_written(written);
    let _ = std::fs::remove_file(pdf);
}

/// At `ask`, nothing is owed to the host until the person says yes.
#[test]
fn at_ask_the_clip_is_owed_only_after_a_yes() {
    let (mut window, pdf) = window("ask", MediaFiles::Ask);
    let document = window.state.documents.focused();
    window.media(document, Some("audio/wav"), embedded(), None);
    let asked = owed(&mut window);
    assert!(
        asked.iter().any(|(kind, _)| *kind == OwedKind::Question),
        "{asked:?}"
    );
    assert!(!asked.iter().any(|(kind, _)| *kind == OwedKind::OpenFile));
    window.answer(true).expect("a question waits");
    let records = owed(&mut window);
    let written = records
        .iter()
        .find(|(kind, _)| *kind == OwedKind::OpenFile)
        .map(|(_, text)| text.clone())
        .expect("owed after the yes");
    remove_written(&written);
    let _ = std::fs::remove_file(pdf);
}

/// A URL is owed as one, and an opener's failure the host reports is said in the window.
#[test]
fn a_url_is_owed_and_a_failure_to_open_it_is_said() {
    let (mut window, pdf) = window("url", MediaFiles::Open);
    let document = window.state.documents.focused();
    window.media(
        document,
        None,
        viewer_core::Media::Url("https://example.org/clip.mp4".to_owned()),
        None,
    );
    let records = owed(&mut window);
    assert!(
        records.iter().any(
            |(kind, text)| *kind == OwedKind::OpenUri && text == "https://example.org/clip.mp4"
        ),
        "{records:?}"
    );
    window.open_failed("https://example.org/clip.mp4", "no application for https");
    let said = owed(&mut window);
    assert!(
        said.iter().any(|(kind, text)| *kind == OwedKind::Said
            && text.contains("did not open it: no application for https")),
        "{said:?}"
    );
    let _ = std::fs::remove_file(pdf);
}

/// §12.6.4.6's launched document goes to the host by its path, never to `xdg-open`.
#[test]
fn a_launched_document_is_owed_to_the_host() {
    let (mut window, pdf) = window("launched", MediaFiles::Ask);
    let letter = pdf.with_extension("txt");
    std::fs::write(&letter, "a letter").expect("the launched file");
    window.hand_launched_over("letter.txt", &letter, "not this sentence");
    let records = owed(&mut window);
    assert!(
        records
            .iter()
            .any(|(kind, text)| *kind == OwedKind::OpenFile && Path::new(text) == letter),
        "{records:?}"
    );
    assert!(
        records.iter().any(|(kind, text)| *kind == OwedKind::Said
            && text.contains("Launch: letter.txt was handed to the program hosting this window")),
        "{records:?}"
    );
    let _ = std::fs::remove_file(letter);
    let _ = std::fs::remove_file(pdf);
}

/// §12.6.4.8's link goes to the host as a `QUORRA_OWED_OPEN_URI` record, never to `xdg-open`,
/// which Windows lacks; a scheme no window hands over is refused before the host is given
/// anything (ADR 3280).
#[test]
fn a_link_is_owed_to_the_host_and_a_scheme_not_handed_over_is_refused() {
    let (mut window, pdf) = window("link", MediaFiles::Ask);
    window.state.links = viewer_host::Links::Open;
    window.follow_the_link("https://example.org/a%20page");
    let records = owed(&mut window);
    assert!(
        records.iter().any(
            |(kind, text)| *kind == OwedKind::OpenUri && text == "https://example.org/a%20page"
        ),
        "{records:?}"
    );
    assert!(
        records
            .iter()
            .any(|(kind, text)| *kind == OwedKind::Said
                && text == "link: https://example.org/a%20page"),
        "{records:?}"
    );
    window.follow_the_link("ftp://example.org/file");
    let refused = owed(&mut window);
    assert!(
        !refused.iter().any(|(kind, _)| *kind == OwedKind::OpenUri),
        "the control: a scheme outside the list reaches no opener: {refused:?}"
    );
    assert!(
        refused.iter().any(|(kind, text)| *kind == OwedKind::Said
            && text.contains("ftp://example.org/file")
            && text.contains("declined")),
        "{refused:?}"
    );
    let _ = std::fs::remove_file(pdf);
}
