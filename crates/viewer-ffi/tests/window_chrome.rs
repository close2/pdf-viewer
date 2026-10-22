//! A document switch must expose effective chrome, not a stale dictionary or another handle.
#![expect(
    clippy::expect_used,
    reason = "fixture failures must fail the regression"
)]
use std::fmt::Write as _;
use std::path::Path;
use std::time::{Duration, Instant};
use viewer_ffi::{OwedKind, Window, WindowChrome};

fn document(path: &Path, catalog: &str) {
    let objects = [
        format!("<< /Type /Catalog /Pages 2 0 R {catalog} >>"),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 40 40] /Resources << >> >>".to_owned(),
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
        .expect("string write");
    }
    let start = pdf.len();
    pdf.push_str("xref\n0 4\n0000000000 65535 f \n");
    for offset in offsets {
        writeln!(pdf, "{offset:010} 00000 n ").expect("string write");
    }
    write!(
        pdf,
        "trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{start}\n%%EOF\n"
    )
    .expect("string write");
    std::fs::write(path, pdf).expect("fixture written");
}

#[track_caller]
fn settles(window: &mut Window, expected: WindowChrome) {
    let until = Instant::now()
        .checked_add(Duration::from_secs(10))
        .expect("bounded deadline");
    while window.chrome() != expected && Instant::now() < until {
        window.tick();
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(window.chrome(), expected);
    let owed = window.take_owed();
    assert!(
        (0..owed.len()).any(|index| owed.kind(index) == Some(OwedKind::Chrome)),
        "the host must be notified to read the changed state"
    );
}

#[test]
fn document_chrome_and_presentation_do_not_leak_across_tabs() {
    let directory = std::env::temp_dir().join(format!("quorra-chrome-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("fixture directory");
    let ordinary = directory.join("ordinary.pdf");
    let hidden = directory.join("hidden.pdf");
    let full = directory.join("full.pdf");
    document(&ordinary, "");
    document(
        &hidden,
        "/ViewerPreferences << /HideMenubar true /HideToolbar true /HideWindowUI true >>",
    );
    document(
        &full,
        "/PageMode /FullScreen /ViewerPreferences << /NonFullScreenPageMode /UseOutlines /HideToolbar true >>",
    );
    let shown = WindowChrome {
        menu_bar: 1,
        tool_bar: 1,
        window_ui: 1,
        other_windows: 1,
    };
    let restricted = WindowChrome {
        menu_bar: 0,
        tool_bar: 0,
        window_ui: 0,
        other_windows: 1,
    };
    let absent = WindowChrome {
        menu_bar: 0,
        tool_bar: 0,
        window_ui: 0,
        other_windows: 0,
    };
    let mut window =
        Window::open(&hidden, None, viewer_host::policy::Words::default()).expect("open");
    window.resize(80, 80, 1.0).expect("resize");
    settles(&mut window, restricted);
    window.open_beside(&ordinary);
    settles(&mut window, shown);
    window.open_beside(&full);
    settles(&mut window, absent);
    for (index, expected) in [(0, restricted), (1, shown), (2, absent), (0, restricted)] {
        window.choose_document(index).expect("existing tab");
        settles(&mut window, expected);
    }
    window.choose_document(2).expect("full screen tab");
    settles(&mut window, absent);
    window.press(viewer_host::Key::Escape, viewer_host::Modifiers::NONE);
    assert_eq!(
        window.chrome(),
        WindowChrome {
            tool_bar: 0,
            ..shown
        }
    );
    let exit = window.take_owed();
    assert!((0..exit.len()).any(|index| exit.chrome_panel(index) == Ok(0)));
    window.press(viewer_host::Key::Escape, viewer_host::Modifiers::NONE);
    let repeated = window.take_owed();
    assert!(!(0..repeated.len()).any(|index| repeated.kind(index) == Some(OwedKind::Chrome)));
    window.choose_document(1).expect("ordinary tab");
    settles(&mut window, shown);
    window.choose_document(2).expect("escaped full-screen tab");
    settles(
        &mut window,
        WindowChrome {
            tool_bar: 0,
            ..shown
        },
    );
    window.press(viewer_host::Key::P, viewer_host::Modifiers::NONE);
    settles(&mut window, absent);
    assert!(!window.close_document());
    settles(&mut window, shown);
    window.choose_document(0).expect("hidden preferences tab");
    settles(&mut window, restricted);
    window.press(viewer_host::Key::P, viewer_host::Modifiers::NONE);
    settles(&mut window, absent);
    window.press(viewer_host::Key::Escape, viewer_host::Modifiers::NONE);
    settles(&mut window, restricted);
    independent_flags(&mut window, &directory, shown);
    drop(window);
    std::fs::remove_dir_all(directory).expect("remove fixtures");
}

fn independent_flags(window: &mut Window, directory: &Path, shown: WindowChrome) {
    for (flag, expected) in [
        (
            "HideMenubar",
            WindowChrome {
                menu_bar: 0,
                ..shown
            },
        ),
        (
            "HideToolbar",
            WindowChrome {
                tool_bar: 0,
                ..shown
            },
        ),
        (
            "HideWindowUI",
            WindowChrome {
                window_ui: 0,
                ..shown
            },
        ),
    ] {
        let path = directory.join(format!("{flag}.pdf"));
        document(&path, &format!("/ViewerPreferences << /{flag} true >>"));
        window.open_beside(&path);
        settles(window, expected);
    }
}
