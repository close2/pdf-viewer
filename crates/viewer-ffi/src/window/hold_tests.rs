//! A zoom command's old picture is held for one refresh, and the window owes one repaint when the
//! refresh ends with the zoomed frame still not there, so that the scaled stand-in takes its place
//! (ADR 2510).
//!
//! The zoomed frame is held out by the test, at `Window::holding`, for as long as the test needs
//! it out, so that the refresh is the only thing that ends the hold (ADR 2437).

use super::{OwedKind, Window};
use std::fmt::Write as _;
use viewer_core::{Command, Zoom};
use viewer_host::native_window::NativeWindow as _;

/// One page, a filled square.
fn document(path: &std::path::Path) {
    let stream = "0 0 20 20 re f";
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
    ];
    for _ in 0..1 {
        objects.push(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] /Resources << >> /Contents 4 0 R >>"
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
            index.checked_add(1).expect("four fixture objects")
        )
        .expect("write");
    }
    let start = pdf.len();
    pdf.push_str("xref\n0 5\n0000000000 65535 f \n");
    for offset in offsets {
        writeln!(pdf, "{offset:010} 00000 n ").expect("write");
    }
    write!(
        pdf,
        "trailer\n<< /Size 5 /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n"
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

/// How many repaints the window owes its caller since it was last asked.
fn repaints(window: &mut Window) -> usize {
    let owed = window.take_owed();
    (0..owed.len())
        .filter(|&index| owed.kind(index) == Some(OwedKind::Repaint))
        .count()
}

#[test]
fn the_end_of_the_refresh_owes_one_repaint_while_the_zoomed_frame_is_not_there() {
    let path = std::env::temp_dir().join(format!("quorra-hold-{}.pdf", std::process::id()));
    document(&path);
    let mut window =
        Window::open(&path, None, viewer_host::policy::Words::default()).expect("open");
    window.resize(400, 400, 1.0).expect("resize");
    rest(&mut window);
    assert!(window.state.hold.wait(std::time::Instant::now()).is_none());

    // A refresh of one hertz is a second, so the hold cannot lapse while the test looks.
    window.refresh_is(1_000);
    window.holding = true;
    window.dispatch(Command::Zoom {
        zoom: Zoom::In,
        at: None,
    });
    let _ = repaints(&mut window);
    assert!(!window.frames().is_empty(), "page one has a frame");
    assert!(
        window.frames().iter().all(|placed| placed.held),
        "the old picture is held where it was"
    );
    window.mind_the_hold();
    assert_eq!(
        repaints(&mut window),
        0,
        "nothing is owed inside the refresh"
    );

    // A kilohertz is a millisecond, which has passed by the time the window looks again.
    window.refresh_is(1_000_000);
    std::thread::sleep(std::time::Duration::from_millis(20));
    window.mind_the_hold();
    assert_eq!(
        repaints(&mut window),
        1,
        "the window is told once that the refresh ended"
    );
    assert!(
        window
            .frames()
            .iter()
            .all(|placed| !placed.held && placed.stands_in()),
        "and it paints the scaled stand-in"
    );
    window.mind_the_hold();
    assert_eq!(repaints(&mut window), 0, "and not again");
    std::fs::remove_file(&path).ok();
}
