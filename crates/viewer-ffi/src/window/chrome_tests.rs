//! Presentation follows the active document without replaying its opening commands.

use super::Window;
use std::fmt::Write as _;
use std::path::Path;
use std::time::{Duration, Instant};
use viewer_core::{Answer, Command, Query};
use viewer_host::native_window::NativeWindow as _;

fn document(path: &Path) {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] /Resources << >> /Dur 0.01 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] /Resources << >> >>",
    ];
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

fn no_panel_command(window: &mut Window) {
    let owed = window.take_owed();
    assert!(!(0..owed.len()).any(|index| owed.chrome_panel(index).is_ok()));
}

#[test]
fn background_open_preserves_clock_and_face_replacement_clears_old_mode() {
    let path = std::env::temp_dir().join(format!("quorra-presentation-{}.pdf", std::process::id()));
    document(&path);
    let mut window =
        Window::open(&path, None, viewer_host::policy::Words::default()).expect("open");
    window.resize(80, 80, 1.0).expect("resize");
    let until = Instant::now()
        .checked_add(Duration::from_secs(10))
        .expect("bounded deadline");
    while window.state.anticipated.is_some() && Instant::now() < until {
        window.tick();
    }
    window.press(viewer_host::Key::P, viewer_host::Modifiers::NONE);
    assert!(window.state.presenting.full_screen());
    let anchor = Instant::now()
        .checked_sub(Duration::from_secs(100))
        .expect("clock anchor");
    window.state.clock = Some(viewer_host::Clock::started(anchor));
    let face = window
        .state
        .shown
        .as_ref()
        .expect("incoming face survived mode synchronization")
        .clone();
    drop(window.take_owed());
    window.owing.owe(super::owed::Owed::Chrome(Some(0)));
    window.choose_document(0).expect("same tab is a no-op");
    window
        .state
        .arrivals
        .wait(viewer_host::Named::file(path.clone()), true);
    window.open_the_next();
    window.settle();
    assert_eq!(window.document_count(), 2);
    assert!(window.state.presenting.full_screen());
    assert_eq!(
        window.state.clock.as_mut().expect("front clock").tick(
            anchor
                .checked_add(Duration::from_secs(1))
                .expect("bounded tick")
        ),
        Some(1000)
    );
    assert!(std::sync::Arc::ptr_eq(
        &face.0,
        &window.state.shown.as_ref().expect("front face").0
    ));
    let owed = window.take_owed();
    assert_eq!(
        (0..owed.len())
            .filter(|index| owed.chrome_panel(*index) == Ok(0))
            .count(),
        1
    );
    window.press(viewer_host::Key::Escape, viewer_host::Modifiers::NONE);
    drop(window.take_owed());
    window.choose_document(1).expect("background tab");
    drop(window.take_owed());
    // An undelivered command from the tab being left cannot become a command for this one.
    window.owing.owe(super::owed::Owed::Chrome(Some(1)));
    window.choose_document(0).expect("escaped original tab");
    assert!(!window.state.presenting.full_screen());
    assert!(window.state.clock.is_none());
    no_panel_command(&mut window);
    window.press(viewer_host::Key::P, viewer_host::Modifiers::NONE);
    let bytes = std::fs::read(&path).expect("replacement bytes");
    let id = window.state.documents.focused();
    window.state.arrivals.offer(
        &mut window.state.documents,
        path.clone(),
        &bytes,
        viewer_core::Purpose::RemoteDocument,
    );
    window.dispatch(Command::Open {
        id,
        bytes: bytes.into(),
        password: None,
        fragment: None,
    });
    assert!(!window.state.presenting.full_screen());
    assert!(window.state.clock.is_none());
    assert!(window.state.arming.is_none());
    assert!(
        window.state.shown.is_some(),
        "replacement's incoming face must survive synchronization"
    );
    // Direct core ticks distinguish an actual Off synchronization from merely changing chrome.
    // The FFI timer does not yet drive presentation clocks; this does not claim that feature.
    window.dispatch(Command::Tick { millis: 1000 });
    assert!(matches!(
        window.state.viewer.query(Query::CurrentPage),
        Answer::Page { index: 0, .. }
    ));
    window.press(viewer_host::Key::P, viewer_host::Modifiers::NONE);
    window.dispatch(Command::Tick { millis: 1000 });
    assert!(matches!(
        window.state.viewer.query(Query::CurrentPage),
        Answer::Page { index: 1, .. }
    ));
    window.press(viewer_host::Key::Escape, viewer_host::Modifiers::NONE);
    incoming_action_clock_survives(&mut window);
    drop(window);
    std::fs::remove_file(path).expect("remove fixture");
}

fn incoming_action_clock_survives(window: &mut Window) {
    // An incoming transition action may arm this after the outgoing activation state was cleared.
    window.state.clock = Some(viewer_host::Clock::for_one_transition(Instant::now()));
    let mut queue = std::collections::VecDeque::new();
    window.synchronize_presentation(&mut queue);
    assert!(window.state.clock.is_some());
    assert!(matches!(
        queue.pop_front(),
        Some(Command::Present(viewer_core::PresentationMode::Off))
    ));
}
