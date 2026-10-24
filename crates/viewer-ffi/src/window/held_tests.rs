//! A question a document raises while the window presents full screen waits until the
//! presentation ends, and a second waits for the first's answer in a window with one dialogue
//! (Table 29, ADR 2309).

use super::{OwedKind, Window};
use std::fmt::Write as _;
use viewer_host::native_window::{Asked, NativeWindow as _};

/// One empty page.
fn document(path: &std::path::Path) {
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

/// How many questions the window owes its caller since it was last asked.
fn questions(window: &mut Window) -> usize {
    let owed = window.take_owed();
    (0..owed.len())
        .filter(|&index| owed.kind(index) == Some(OwedKind::Question))
        .count()
}

#[test]
fn a_question_raised_while_presenting_waits_until_the_presentation_ends() {
    let path = std::env::temp_dir().join(format!("quorra-held-{}.pdf", std::process::id()));
    document(&path);
    let mut window =
        Window::open(&path, None, viewer_host::policy::Words::default()).expect("open");
    window.resize(400, 400, 1.0).expect("resize");
    window.presentation(false);
    assert!(window.state.presenting.full_screen());
    let question = viewer_host::Question {
        reasons: "a document asks".to_owned(),
        choice: "go ahead, or do not".to_owned(),
    };
    let first = Asked::Link {
        uri: "https://example.org/first".to_owned(),
    };
    window.ask(first, &question);
    window.ask(
        Asked::Link {
            uri: "https://example.org/second".to_owned(),
        },
        &question,
    );
    assert_eq!(questions(&mut window), 0, "nothing over the presentation");
    assert_eq!(window.state.pump.held(), 2);
    assert!(
        window
            .status()
            .contains("waits until the presentation ends"),
        "{}",
        window.status()
    );

    window.presentation(true);
    assert!(!window.state.presenting.full_screen());
    assert_eq!(
        questions(&mut window),
        1,
        "the first, once the presentation ended"
    );
    assert_eq!(
        window.state.pump.held(),
        1,
        "one dialogue: the second waits for the first's answer"
    );
    window.answer(false).expect("a question was waiting");
    assert_eq!(questions(&mut window), 1, "the second follows the answer");
    assert_eq!(window.state.pump.held(), 0);
    std::fs::remove_file(&path).expect("the fixture is removed");
}

#[test]
fn a_question_raised_while_not_presenting_is_put_at_once() {
    let path = std::env::temp_dir().join(format!("quorra-unheld-{}.pdf", std::process::id()));
    document(&path);
    let mut window =
        Window::open(&path, None, viewer_host::policy::Words::default()).expect("open");
    window.resize(400, 400, 1.0).expect("resize");
    let _ = window.take_owed();
    window.ask(
        Asked::Link {
            uri: "https://example.org/".to_owned(),
        },
        &viewer_host::Question {
            reasons: "a document asks".to_owned(),
            choice: "go ahead, or do not".to_owned(),
        },
    );
    assert_eq!(questions(&mut window), 1);
    assert_eq!(window.state.pump.held(), 0);
    std::fs::remove_file(&path).expect("the fixture is removed");
}
