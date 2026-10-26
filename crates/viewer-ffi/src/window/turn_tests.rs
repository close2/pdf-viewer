//! A page turn's key returns with the page's read out on a thread of its own, the window goes on
//! answering, and a second turn waits for the first's read (ADR 2344).

use super::Window;
use std::fmt::Write as _;
use viewer_core::{Answer, Command, PageTarget, Query};
use viewer_host::native_window::NativeWindow as _;

/// Three pages, each a filled square.
fn document(path: &std::path::Path) {
    let stream = "0 0 20 20 re f";
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R] /Count 3 >>".to_owned(),
    ];
    for _ in 0..3 {
        objects.push(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] /Resources << >> /Contents 6 0 R >>"
                .to_owned(),
        );
    }
    objects.push(format!(
        "<< /Length {} >>\nstream\n{stream}\nendstream",
        stream.len()
    ));
    let mut pdf = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        writeln!(
            pdf,
            "{} 0 obj\n{object}\nendobj",
            index.checked_add(1).expect("six fixture objects")
        )
        .expect("write");
    }
    let start = pdf.len();
    pdf.push_str("xref\n0 7\n0000000000 65535 f \n");
    for offset in offsets {
        writeln!(pdf, "{offset:010} 00000 n ").expect("write");
    }
    write!(
        pdf,
        "trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n"
    )
    .expect("write");
    std::fs::write(path, pdf).expect("fixture");
}

/// Ticks the window until nothing beside it is running, as a caller's loop does.
fn rest(window: &mut Window) {
    for _ in 0..10_000 {
        if window.interval().is_none() {
            return;
        }
        window.tick();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    panic!("the window never came to rest");
}

/// The pages the viewer holds a frame of.
fn framed(window: &Window) -> Vec<usize> {
    match window.state.viewer.query(Query::Frame) {
        Answer::Frame(frames) => frames.iter().map(|frame| frame.page).collect(),
        _ => Vec::new(),
    }
}

#[test]
fn a_turn_returns_with_its_read_out_and_a_second_turn_waits_for_it() {
    let path = std::env::temp_dir().join(format!("quorra-turn-{}.pdf", std::process::id()));
    document(&path);
    let mut window =
        Window::open(&path, None, viewer_host::policy::Words::default()).expect("open");
    window.resize(400, 400, 1.0).expect("resize");
    rest(&mut window);
    assert_eq!(framed(&window), [0], "page one is on the screen");

    window.dispatch(Command::GoTo(PageTarget::Next));
    assert!(
        window.state.turning.is_some(),
        "the key's handler returned with page two's read out"
    );
    assert!(
        framed(&window).is_empty(),
        "and with no frame of page two yet: the window shows what it retains"
    );
    assert!(window.interval().is_some(), "the window looks for the read");

    window.dispatch(Command::GoTo(PageTarget::Next));
    assert_eq!(
        window.state.behind_the_turn.len(),
        1,
        "a second turn waits for the first's read"
    );

    rest(&mut window);
    assert!(window.state.turning.is_none() && window.state.behind_the_turn.is_empty());
    assert_eq!(
        framed(&window),
        [2],
        "both turns were taken, in order, and page three's frame landed"
    );
    std::fs::remove_file(&path).ok();
}
