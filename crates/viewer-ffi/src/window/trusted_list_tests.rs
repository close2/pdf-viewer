//! Shift + T reaches the trusted lists' fetch at each of the network level's answers, as every
//! window carries it out (`viewer_host::WindowAct::TrustedLists`, ADR 2886).
//!
//! What a refresh fetches and keeps is ETSI TS 119 615 clauses 4.1 and 4.2's, pinned in
//! `pdf-signature` and through `viewer-host`'s `Refresher`; what these hold is the window's half:
//! `refuse` says so and sends nothing, `ask` owes a question naming the directory and fetches only
//! on a yes, and `send` fetches — over a listener on this machine's loopback, through the submit
//! client — with the refresh's sentences said once its thread is in. `doc/questions/A98` set the
//! four levels for every fetch under `--submissions=`, and ADR 2882 put the lists under it.

use std::fmt::Write as _;
use std::io::{BufRead as _, BufReader, Write as _};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use viewer_host::{ActLevel, Key, Modifiers, Submissions};

use super::{ActKind, OwedKind, Window};

#[path = "../../../../tools/conformance/test-support/signed_under_a_policy.rs"]
mod signed_under_a_policy;

/// One empty page, `held_tests`' fixture.
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

/// A fresh directory of trusted lists for one test: the Official Journal's certificate file —
/// X.690's `SEQUENCE` tag is what makes it read as DER — and `OJEU-LOTL-Loc` naming `lotl`.
fn configured(name: &str, lotl: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "quorra-trusted-lists-{}-{name}",
        std::process::id()
    ));
    // A directory left by an earlier run is the one thing to clear; its absence is the usual case.
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a temporary directory");
    std::fs::write(directory.join("OJEU-LOTL-Certs-Set.der"), [0x30, 0x00])
        .expect("the certificate file");
    std::fs::write(
        directory.join(pdf_signature::trusted_list::authentication::OJEU_LOTL_LOC),
        lotl,
    )
    .expect("the location file");
    directory
}

/// A window on one empty page whose reader named `directory` with `--trusted-lists`, at `level`.
fn window(name: &str, directory: Option<&Path>, level: Submissions) -> (Window, PathBuf) {
    window_on(name, directory, level, document)
}

/// [`window`] on the document `write` puts at the path it is given.
fn window_on(
    name: &str,
    directory: Option<&Path>,
    level: Submissions,
    write: fn(&Path),
) -> (Window, PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "quorra-trusted-lists-{}-{name}.pdf",
        std::process::id()
    ));
    write(&path);
    let mut words = viewer_host::policy::Words::default();
    if let Some(directory) = directory {
        words
            .take(
                std::ffi::OsStr::new(viewer_host::policy::TRUSTED_LISTS),
                &mut std::iter::once(directory.as_os_str().to_owned()),
            )
            .expect("the directory is one");
    }
    let mut window = Window::open(&path, None, words).expect("open");
    window.resize(400, 400, 1.0).expect("resize");
    window.state.restrictions = window.state.restrictions.with(ActLevel::Submissions(level));
    let _ = window.take_owed();
    (window, path)
}

/// Every sentence and every question the window owes since it was last asked, in order.
fn owed(window: &mut Window) -> (Vec<String>, Vec<String>) {
    let owed = window.take_owed();
    let mut said = Vec::new();
    let mut asked = Vec::new();
    for index in 0..owed.len() {
        let text = owed.text(index).map(str::to_owned);
        match (owed.kind(index), text) {
            (Some(OwedKind::Said), Ok(text)) => said.push(text),
            (Some(OwedKind::Question), Ok(text)) => asked.push(text),
            _ => {}
        }
    }
    (said, asked)
}

/// Shift + T, as `quorra_window_press` takes it.
fn press(window: &mut Window) -> (ActKind, f32) {
    window.press(Key::T, Modifiers::SHIFT)
}

/// Ticks until the refresh's thread is in, as a caller's timer would, and answers what was said.
fn refreshed(window: &mut Window) -> Vec<String> {
    let mut said = Vec::new();
    for _ in 0..400 {
        window.tick();
        said.extend(owed(window).0);
        if window.state.refresher.interval().is_none() {
            return said;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(
        window.state.refresher.interval().is_none(),
        "the refresh did not come back: {said:?}"
    );
    said
}

/// A listener that answers one request with a 404, and hands back the request line.
fn serve_once() -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("the loopback takes a listener");
    let port = listener.local_addr().expect("a bound port").port();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("one connection arrives");
        let mut reader = BufReader::new(stream.try_clone().expect("the stream clones"));
        let mut line = String::new();
        reader.read_line(&mut line).expect("a request line");
        loop {
            let mut header = String::new();
            reader.read_line(&mut header).expect("a header line");
            if header.trim_end().is_empty() {
                break;
            }
        }
        let mut stream = stream;
        // A client that refused the answer part-way closes the connection; the write that fails
        // is that refusal arriving rather than a fault in the test.
        let _ = write!(
            stream,
            "HTTP/1.1 404 Not Found\r\nContent-Length: 4\r\nConnection: close\r\n\r\ngone"
        );
        line.trim_end().to_owned()
    });
    (format!("http://127.0.0.1:{port}"), server)
}

/// `refuse` sends nothing and says why, and the window is owed no act: the key is the library's
/// to carry out, so a C caller has nothing to do but say the sentence.
#[test]
fn refuse_sends_nothing_and_says_why() {
    let directory = configured("refuse", "http://127.0.0.1:9/never-asked.xml");
    let (mut window, path) = window("refuse", Some(&directory), Submissions::Refuse);
    assert_eq!(press(&mut window), (ActKind::Nothing, 0.0));
    let (said, asked) = owed(&mut window);
    assert!(asked.is_empty(), "{asked:?}");
    assert!(
        said.iter()
            .any(|line| line.starts_with("trusted lists: not fetched")
                && line.contains("send nothing to a server")),
        "{said:?}"
    );
    assert!(window.state.refresher.interval().is_none(), "no thread");
    std::fs::remove_file(&path).expect("the fixture is removed");
}

/// No directory named is refused at every level, `send` included, before the level is read: there
/// is nowhere to keep the lists (ADR 2882).
#[test]
fn no_directory_is_refused_even_at_send() {
    let (mut window, path) = window("none", None, Submissions::Send);
    assert_eq!(press(&mut window), (ActKind::Nothing, 0.0));
    let (said, _) = owed(&mut window);
    assert!(
        said.iter()
            .any(|line| line.contains("not fetched") && line.contains("--trusted-lists")),
        "{said:?}"
    );
    assert!(window.state.refresher.interval().is_none(), "no thread");
    std::fs::remove_file(&path).expect("the fixture is removed");
}

/// `ask` owes one question naming the directory, and a `no` fetches nothing and says so — the
/// listener is never connected to, which the window's having no thread out shows.
#[test]
fn ask_puts_the_question_and_a_no_fetches_nothing() {
    let directory = configured("ask-no", "http://127.0.0.1:9/never-asked.xml");
    let (mut window, path) = window("ask-no", Some(&directory), Submissions::Ask);
    assert_eq!(press(&mut window), (ActKind::Nothing, 0.0));
    let (_, asked) = owed(&mut window);
    assert_eq!(asked.len(), 1, "{asked:?}");
    assert!(
        asked[0].contains(&directory.display().to_string()),
        "{asked:?}"
    );
    assert!(window.state.refresher.interval().is_none(), "nothing yet");
    window.answer(false).expect("a question was waiting");
    let (said, _) = owed(&mut window);
    assert!(
        said.iter()
            .any(|line| line.contains("not fetched")
                && line.contains(viewer_host::restriction::DO_NOT)),
        "{said:?}"
    );
    assert!(window.state.refresher.interval().is_none(), "no thread");
    std::fs::remove_file(&path).expect("the fixture is removed");
}

/// `ask` answered yes fetches: TS 119 615 PRO-4.1.4-01's GET goes to `OJEU-LOTL-Loc`, and the 404
/// it gets is said once the thread is in, with the directory as it was.
#[test]
fn ask_answered_yes_fetches_from_the_configured_location() {
    let (base, server) = serve_once();
    let directory = configured("ask-yes", &format!("{base}/eu-lotl.xml"));
    let (mut window, path) = window("ask-yes", Some(&directory), Submissions::Ask);
    press(&mut window);
    window.answer(true).expect("a question was waiting");
    let said = refreshed(&mut window);
    assert_eq!(
        server.join().expect("the server ran"),
        "GET /eu-lotl.xml HTTP/1.1"
    );
    assert!(
        said.iter()
            .any(|line| line.contains("nothing was kept") && line.contains("answered 404")),
        "{said:?}"
    );
    std::fs::remove_file(&path).expect("the fixture is removed");
}

/// `send` fetches without a question, the refresh's start said first and its outcome after.
#[test]
fn send_fetches_without_asking() {
    let (base, server) = serve_once();
    let directory = configured("send", &format!("{base}/eu-lotl.xml"));
    let (mut window, path) = window("send", Some(&directory), Submissions::Send);
    assert_eq!(press(&mut window), (ActKind::Nothing, 0.0));
    let (said, asked) = owed(&mut window);
    assert!(asked.is_empty(), "{asked:?}");
    assert!(
        said.iter()
            .any(|line| line.starts_with("trusted lists: fetching")),
        "{said:?}"
    );
    let said = refreshed(&mut window);
    assert_eq!(
        server.join().expect("the server ran"),
        "GET /eu-lotl.xml HTTP/1.1"
    );
    assert!(
        said.iter().any(|line| line.contains("answered 404")),
        "{said:?}"
    );
    std::fs::remove_file(&path).expect("the fixture is removed");
}

/// The signed document `signed_under_a_policy` generates.
fn signed(path: &Path) {
    std::fs::write(path, signed_under_a_policy::document()).expect("fixture");
}

/// **A signed document's policy reference reaches the refresh through Shift + T** (ADR 2936,
/// ETSI TS 119 172-2 clause 4.22.2.1 b)): with the policy's copy bound, `ask` names the list the
/// policy references in its question, and the yes starts the refresh saying it fetches that one
/// list besides the Member States'. The calibration is the same document before its copy is bound,
/// whose question names no reference: only a bound policy is anybody's agreement.
#[test]
fn a_bound_policy_s_reference_goes_with_shift_t_to_the_refresh() {
    let directory = configured("referenced", "http://127.0.0.1:9/never-asked.xml");
    let (mut window, path) = window_on("referenced", Some(&directory), Submissions::Ask, signed);
    // The page is read on a thread of its own after the resize, and the report has nothing to say
    // about the file until it is in, so the window is ticked as a caller's timer would tick it.
    for _ in 0..400 {
        window.tick();
        let read = window
            .state
            .viewer
            .handle(viewer_core::Command::Report)
            .any(|event| matches!(event, viewer_core::Event::SignaturePoliciesPublished { .. }));
        if read {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    // The window's own report, once the read is in, asks whether to fetch the policy the signature
    // names, and Shift + T is declined while that question waits; the person answers it no.
    let _ = window.answer(false);
    let _ = owed(&mut window);
    press(&mut window);
    let (said, asked) = owed(&mut window);
    assert_eq!(asked.len(), 1, "{said:?}");
    assert!(
        !asked[0].contains(signed_under_a_policy::REFERENCE),
        "an unbound policy is nobody's agreement: {asked:?}"
    );
    window.answer(false).expect("a question was waiting");
    let _ = owed(&mut window);

    let document = window.state.documents.focused();
    assert_eq!(
        signed_under_a_policy::bind(&mut window.state.viewer, document),
        1,
        "the signature publishes its policy"
    );
    press(&mut window);
    let (_, asked) = owed(&mut window);
    assert_eq!(asked.len(), 1, "{asked:?}");
    assert!(
        asked[0].contains(signed_under_a_policy::REFERENCE),
        "the question names the referenced list: {asked:?}"
    );
    window.answer(true).expect("a question was waiting");
    let (said, _) = owed(&mut window);
    assert!(
        said.iter()
            .any(|line| line.starts_with("trusted lists: fetching")
                && line.contains("the 1 list(s) a signature policy references")),
        "{said:?}"
    );
    let _ = refreshed(&mut window);
    std::fs::remove_file(&path).expect("the fixture is removed");
}
