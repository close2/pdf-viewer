//! `quorra-transform archive --to <first> --or-else-to <second>`: one request, two targets, the
//! second written only where the first refuses (`doc/adr/2498`).
//!
//! The first target is PDF/A-2a, which an untagged page can only meet by being given a structure
//! tree, and inventing one is inventing semantics the converter will not (`doc/pdf-a-conversion-limits.md`
//! section 5.1, `doc/rfc/0006` section 5.7); the second is PDF/A-2b, which the same page meets.

use super::*;
use std::path::{Path, PathBuf};
use std::process::Command;

/// A fresh directory for one test, named by process and counter.
fn scratch() -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("or-else-to-{}-{serial}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the temporary directory is writable");
    dir
}

/// Runs the program in `dir`, answering the exit status and standard error.
fn run(dir: &Path, arguments: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_quorra-transform"))
        .args(arguments)
        .current_dir(dir)
        .output()
        .expect("the program runs");
    (
        output.status.code().expect("the program exited"),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// An untagged page that meets PDF/A-2b and not 2a.
fn an_untagged_page() -> Vec<u8> {
    Conforming {
        resources: "/ColorSpace << /CS0 [/CalGray << /WhitePoint [0.9505 1.0 1.089] >>] >>"
            .to_owned(),
        contents: Some((String::new(), b"/CS0 cs 0 sc 10 10 50 50 re f".to_vec())),
        ..Conforming::part_two()
    }
    .build()
}

/// The identification schema's `pdfaid:part` and `pdfaid:conformance` as the file states them.
fn identification(bytes: &[u8]) -> (String, String) {
    let text = String::from_utf8_lossy(bytes);
    let value = |name: &str| -> String {
        let element = format!("<pdfaid:{name}>");
        let after = text.split_once(&element).expect("the packet states it").1;
        after
            .split_once('<')
            .expect("the element closes")
            .0
            .to_owned()
    };
    (value("part"), value("conformance"))
}

#[test]
fn the_first_target_refusing_writes_the_second_and_the_report_says_so() {
    let dir = scratch();
    std::fs::write(dir.join("in.pdf"), an_untagged_page()).expect("writable");
    let (status, _, stderr) = run(
        &dir,
        &["archive", "in.pdf", "--to", "2a", "-o", "alone.pdf"],
    );
    assert_eq!(status, 4, "2a alone refuses: {stderr}");
    assert!(!dir.join("alone.pdf").exists());

    let (status, _, stderr) = run(
        &dir,
        &[
            "archive",
            "in.pdf",
            "--to",
            "2a",
            "--or-else-to",
            "2b",
            "-o",
            "out.pdf",
        ],
    );
    assert_eq!(status, 0, "{stderr}");
    let written = std::fs::read(dir.join("out.pdf")).expect("the second target is written");
    assert_eq!(identification(&written), ("2".to_owned(), "B".to_owned()));
    assert!(
        stderr.contains("PDF/A-2a was asked for first and refused"),
        "{stderr}"
    );
    assert!(stderr.contains("PDF/A-2b next"), "{stderr}");

    let (status, stdout, stderr) = run(
        &dir,
        &[
            "archive",
            "in.pdf",
            "--to",
            "2a",
            "--or-else-to",
            "2b",
            "--report",
            "json",
            "-o",
            "json.pdf",
        ],
    );
    assert_eq!(status, 0, "{stderr}");
    assert!(
        stdout.contains("\"refused_first\": {") && stdout.contains("\"requested\": \"PDF/A-2a\""),
        "{stdout}"
    );
}

#[test]
fn the_first_target_writing_leaves_the_second_unasked() {
    let dir = scratch();
    std::fs::write(dir.join("in.pdf"), an_untagged_page()).expect("writable");
    let (status, _, stderr) = run(
        &dir,
        &[
            "archive",
            "in.pdf",
            "--to",
            "2b",
            "--or-else-to",
            "4",
            "-o",
            "out.pdf",
        ],
    );
    assert_eq!(status, 0, "{stderr}");
    let written = std::fs::read(dir.join("out.pdf")).expect("the first target is written");
    assert_eq!(identification(&written).0, "2");
    assert!(!stderr.contains("asked for first"), "{stderr}");
}

#[test]
fn both_targets_refusing_lists_each_targets_refusals_and_exits_refused() {
    let dir = scratch();
    let source = Conforming {
        objects: vec![
            String::from_utf8_lossy(&stream(
                "/Type /XObject /Subtype /Image /Width 1 /Height 1 \
             /ColorSpace [/CalGray << /WhitePoint [0.9505 1.0 1.089] >>] \
             /BitsPerComponent 8 /Interpolate true /Length 1",
                b"\x00",
            ))
            .into_owned(),
        ],
        resources: "/XObject << /Im0 6 0 R >>".to_owned(),
        contents: Some((String::new(), b"q 10 0 0 10 0 0 cm /Im0 Do Q".to_vec())),
        ..Conforming::part_two()
    }
    .build();
    std::fs::write(dir.join("in.pdf"), source).expect("writable");
    let (status, _, stderr) = run(
        &dir,
        &[
            "archive",
            "in.pdf",
            "--to",
            "2b",
            "--or-else-to",
            "4",
            "-o",
            "out.pdf",
        ],
    );
    assert_eq!(status, 4, "{stderr}");
    assert!(!dir.join("out.pdf").exists());
    assert!(stderr.contains("refused: at PDF/A-2b: "), "{stderr}");
    assert!(stderr.contains("refused: at PDF/A-4: "), "{stderr}");
}

#[test]
fn a_second_target_that_is_the_first_or_unknown_is_a_usage_error() {
    let dir = scratch();
    std::fs::write(dir.join("in.pdf"), an_untagged_page()).expect("writable");
    for word in ["2b", "9z"] {
        let (status, _, stderr) = run(
            &dir,
            &[
                "archive",
                "in.pdf",
                "--to",
                "2b",
                "--or-else-to",
                word,
                "-o",
                "out.pdf",
            ],
        );
        assert_eq!(status, 1, "{word}: {stderr}");
        assert!(stderr.contains("--or-else-to"), "{word}: {stderr}");
    }
}

#[test]
fn the_flag_belongs_to_archive() {
    let dir = scratch();
    std::fs::write(dir.join("in.pdf"), an_untagged_page()).expect("writable");
    let (status, _, stderr) = run(
        &dir,
        &["render", "in.pdf", "--or-else-to", "4", "-o", "out-%d.png"],
    );
    assert_eq!(status, 1, "{stderr}");
    assert!(stderr.contains("belongs to archive"), "{stderr}");
}
