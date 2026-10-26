//! The window's tests: what its password, restart and question paths do without a window.
//!
//! Moved out of the dispatch file with `Host`'s methods (ADR 2352).

use std::path::PathBuf;
use std::time::Instant;

use viewer_confined::ConfinedError;
use viewer_core::{DocumentId, Event, Viewing, Zoom};
use winit::keyboard::{Key, NamedKey};

use super::Host;

/// A host with no window, no surface and no worker: everything the password path touches
/// short of a pipe, which is exactly the part this file added.
fn a_host() -> Host {
    // `--cpu`'s shape: a host with no instance thread, which is what a test wants — the
    // password path under test touches no surface either way.
    Host::new(PathBuf::from("locked.pdf"), 0, true, false, Instant::now())
}

/// §7.6.4.1's event puts the card up instead of ending the document.
///
/// The behaviour this replaces was a `stop`: before ADR 0718 the window declined every
/// encrypted document by name, so the discriminating assertion is `stopped.is_none()`.
#[test]
fn an_encrypted_document_is_prompted_for_not_refused() {
    let mut host = a_host();
    host.event(Event::PasswordRequired {
        document: DocumentId(0),
    });
    assert!(host.password.shown, "the card is up");
    assert!(host.stopped.is_none(), "the document was not given up on");
}

/// The attempts exhaust into a sentence and an open window — `viewer_host::password`'s rule
/// that no host may make `Ask::Exhausted` mean the window closes.
#[test]
fn exhausted_attempts_leave_the_window_open() {
    let mut host = a_host();
    for _ in 0..=viewer_host::password::ATTEMPTS {
        host.event(Event::PasswordRequired {
            document: DocumentId(0),
        });
        // A new `PasswordRequired` only ever follows an answered card (the retry's failed
        // open), so between events the card is answered — what was typed is not the point.
        if host.password.shown {
            let _ = host.password.take();
        }
    }
    assert!(!host.password.shown, "it stopped asking");
    assert!(host.stopped.is_none(), "and the window is still a window");
    assert!(
        host.heading.contains(viewer_host::password::EXHAUSTED),
        "the sentence is where a person will read it, was {:?}",
        host.heading
    );
}

/// Escape while the card is up declines the prompt; it is not [`Host::abort`].
///
/// The distinction is the whole reason `password_key` runs before the page keys: an Escape
/// that killed the worker would turn "I don't know the password" into "end the program's
/// document", two different facts about the reader.
#[test]
fn escape_declines_the_prompt_without_aborting() {
    let mut host = a_host();
    host.event(Event::PasswordRequired {
        document: DocumentId(0),
    });
    host.password_key(&Key::Named(NamedKey::Escape));
    assert!(!host.password.shown, "the card came down");
    assert!(host.stopped.is_none(), "nothing was aborted");
    assert!(
        host.heading.contains(viewer_host::password::CANCELLED),
        "the decline is said, was {:?}",
        host.heading
    );
}

/// A file that has gone away between the first open and the retry is a fact about this
/// machine: said in the heading, and the window survives it (trap 5, the flagship's rule).
#[test]
fn a_file_gone_before_the_retry_is_said_not_fatal() {
    let mut host = Host::new(
        PathBuf::from("tests/does-not-exist-781.pdf"),
        0,
        true,
        false,
        Instant::now(),
    );
    host.event(Event::PasswordRequired {
        document: DocumentId(0),
    });
    host.password_key(&Key::Character("x"));
    host.password_key(&Key::Named(NamedKey::Enter));
    assert!(!host.password.shown, "the card came down");
    assert!(host.stopped.is_none(), "the window is still a window");
    assert!(
        host.heading.contains("cannot reopen"),
        "the missing file is said, was {:?}",
        host.heading
    );
}

/// A worker that died leaves the window open with a restart owed, and the restart goes back
/// to the whole view the reader was looking at.
///
/// The behaviour this replaces is a `stop`: from ADR 0713 until ADR 0734 every death ended the
/// document, so the discriminating assertions are `stopped.is_none()` and the view. **The
/// magnification and the offset are in it since ADR 0737**, and they are what the assertion is
/// really about: until then a restart went back to the page and to the document's opening
/// view, which is the sentence this window used to print.
#[test]
fn a_dead_worker_leaves_a_restart_owed_at_the_readers_view() {
    let mut host = a_host();
    let was = Viewing {
        page: 4,
        zoom: Zoom::Scale(2.75),
        scroll: (13.0, 907.5),
    };
    host.event(Event::PageChanged {
        document: DocumentId(0),
        index: 4,
        label: None,
        of: 9,
        section: None,
    });
    host.resuming.showing(was);
    host.died(&ConfinedError::WorkerDied {
        detail: "killed by signal 6".to_owned(),
    });
    assert!(host.stopped.is_none(), "the document was not given up on");
    assert_eq!(
        host.resume.map(|reopen| reopen.view),
        Some(was),
        "the restart goes back to where the reader was, magnification and offset included"
    );
    assert!(
        host.heading.contains("restarting"),
        "and the window says so, was {:?}",
        host.heading
    );
}

/// A directory of its own for one test, holding the document and the file it names.
fn beside(test: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("quorra-confined-{}-{test}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("a scratch directory");
    std::fs::write(directory.join("other.pdf"), b"%PDF-2.0\n").expect("the named file");
    directory
}

/// §12.6.4.3's question for a file beside the document, as the worker raises it.
fn needs_other() -> Event {
    Event::NeedsFile {
        document: DocumentId(0),
        purpose: viewer_core::Purpose::RemoteDocument,
        name: "other.pdf".to_owned(),
        beside: false,
    }
}

/// The *ask* level's question reaches this window as the toolkits' does: a link puts the card
/// up and Escape takes it down with nothing handed over (ADR 2040). Before it the window was
/// pinned to `refuse`, so the discriminating assertion is that the card was shown at all.
#[test]
fn a_link_at_ask_puts_the_question_and_escape_declines_it() {
    let mut host = a_host();
    host.event(Event::OpenUri {
        document: DocumentId(0),
        uri: "https://example.org/linked".to_owned(),
    });
    assert!(host.question.shown, "the question is up");
    host.press(&Key::Named(NamedKey::Escape));
    assert!(!host.question.shown, "the card came down");
    assert!(host.asked.is_none(), "and nothing is still asked");
    assert!(
        host.stopped.is_none(),
        "Escape on the card is not the abort"
    );
}

/// At `refuse` nothing is asked: the level is the reader's, and the card is the *ask* level's.
#[test]
fn a_link_at_refuse_is_not_asked() {
    let mut host = a_host();
    host.links = viewer_host::Links::Refuse;
    host.event(Event::OpenUri {
        document: DocumentId(0),
        uri: "https://example.org/linked".to_owned(),
    });
    assert!(!host.question.shown, "no question at refuse");
}

/// A file a document named is asked about, read on a `yes`, and becomes the window's document
/// when the worker says it opened — the title, the save and a resume all follow it.
#[test]
fn a_remote_file_answered_yes_becomes_the_windows_document() {
    let directory = beside("yes");
    let mut host = a_host();
    host.path = directory.join("document.pdf");
    host.event(needs_other());
    assert!(host.question.shown, "the question is up");
    host.press(&Key::Named(NamedKey::Enter));
    assert!(!host.question.shown, "the card came down");
    assert!(host.offered.is_some(), "the file was read and offered");
    host.event(Event::Opened {
        document: DocumentId(0),
        pages: 1,
    });
    assert_eq!(host.path, directory.join("other.pdf"));
    assert!(
        host.heading.starts_with("other.pdf"),
        "was {:?}",
        host.heading
    );
    std::fs::remove_dir_all(&directory).expect("the scratch directory goes");
}

/// An offer nothing took is spent once its command is answered: a later open of the window's
/// own document is not mistaken for it.
#[test]
fn an_offer_nothing_took_is_spent() {
    let directory = beside("spent");
    let mut host = a_host();
    host.path = directory.join("document.pdf");
    host.remote_documents = viewer_host::RemoteDocuments::Open;
    host.event(needs_other());
    assert!(host.offered.is_some(), "read and offered at open, unasked");
    host.supplied();
    host.event(Event::Opened {
        document: DocumentId(0),
        pages: 1,
    });
    assert_eq!(host.path, directory.join("document.pdf"));
    std::fs::remove_dir_all(&directory).expect("the scratch directory goes");
}

/// A file a document named that asks for §7.6.4.1's password is the file the card is about,
/// and a cancelled prompt declines it rather than the window's own document.
#[test]
fn a_locked_remote_file_is_the_prompts_and_a_cancel_declines_it() {
    let directory = beside("locked");
    let mut host = a_host();
    host.path = directory.join("document.pdf");
    host.remote_documents = viewer_host::RemoteDocuments::Open;
    host.event(needs_other());
    host.event(Event::PasswordRequired {
        document: DocumentId(0),
    });
    assert!(host.password.shown, "the prompt is up");
    assert_eq!(host.prompted_for(), directory.join("other.pdf"));
    host.password_key(&Key::Named(NamedKey::Escape));
    assert!(host.prompting.is_none(), "the hold was declined");
    assert_eq!(
        host.path,
        directory.join("document.pdf"),
        "the window's own stays"
    );
    assert!(
        !host.heading.contains(viewer_host::password::CANCELLED),
        "the window's own document was not the one cancelled, was {:?}",
        host.heading
    );
    std::fs::remove_dir_all(&directory).expect("the scratch directory goes");
}

/// One question stands at a time: a second that arrives while one is up is declined, and the
/// first is still the one the card holds.
#[test]
fn a_second_question_is_declined_while_one_is_up() {
    let directory = beside("second");
    let mut host = a_host();
    host.path = directory.join("document.pdf");
    host.event(Event::OpenUri {
        document: DocumentId(0),
        uri: "https://example.org/linked".to_owned(),
    });
    host.event(needs_other());
    assert!(
        matches!(host.asked, Some(super::asking::Asked::Link { .. })),
        "the link's question is still the one up"
    );
    std::fs::remove_dir_all(&directory).expect("the scratch directory goes");
}

/// §12.7.6.2's request as the worker raises it, to `url`.
fn submits_to(url: &str) -> Event {
    Event::Submit {
        document: DocumentId(0),
        submission: Box::new(pdf_model::submission::Submission {
            url: url.to_owned(),
            method: pdf_model::submission::Method::Post,
            format: pdf_model::submission::Format::Fdf,
            media_type: "application/vnd.fdf".to_owned(),
            body: b"%FDF-1.2\n".to_vec(),
            fields: 1,
            owed: Vec::new(),
        }),
    }
}

/// A listener on this machine that answers one request with §12.7.8.1's media type, and hands
/// back the request's first line — the transmission under test goes nowhere else.
fn one_answer() -> (String, std::thread::JoinHandle<String>) {
    use std::io::{BufRead as _, Read as _, Write as _};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let port = listener.local_addr().expect("its address").port();
    let served = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("the window's request");
        let mut reading = std::io::BufReader::new(stream.try_clone().expect("a second handle"));
        let mut first = String::new();
        reading.read_line(&mut first).expect("the request line");
        let mut length = 0usize;
        loop {
            let mut header = String::new();
            reading.read_line(&mut header).expect("a header");
            if header.trim().is_empty() {
                break;
            }
            if let Some(value) = header.to_ascii_lowercase().strip_prefix("content-length:") {
                length = value.trim().parse().expect("a length");
            }
        }
        let mut body = vec![0; length];
        reading.read_exact(&mut body).expect("the body");
        let answer = b"%FDF-1.2\n";
        let mut stream = stream;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/vnd.fdf\r\nContent-Length: {}\r\n\
             Connection: close\r\n\r\n",
            answer.len()
        )
        .expect("the answer's head");
        stream.write_all(answer).expect("the answer's body");
        first
    });
    (format!("http://127.0.0.1:{port}/submit"), served)
}

/// The answers that come back within ten seconds, collected as the loop collects them.
fn answers_of(host: &mut Host) -> usize {
    let began = Instant::now();
    let mut arrived = 0usize;
    while host.submitter.interval().is_some() && began.elapsed().as_secs() < 10 {
        std::thread::sleep(viewer_host::submit::LOOK);
        arrived = arrived.saturating_add(host.submitter.collect().len());
    }
    arrived
}

/// At the default level, `ask`, a form goes on the card, and Enter sends it from this side:
/// the listener reads a POST and the answer comes back to be handed to the worker (ADR 2076).
/// Before it the window was pinned to `refuse`, so the discriminating assertions are the card
/// and the request the listener read.
#[test]
fn a_form_at_ask_is_put_on_the_card_and_sent_on_a_yes() {
    let (url, served) = one_answer();
    let mut host = a_host();
    host.event(submits_to(&url));
    assert!(host.question.shown, "the question is up");
    assert!(
        host.submitter.interval().is_none(),
        "nothing is sent while it stands"
    );
    host.press(&Key::Named(NamedKey::Enter));
    assert!(!host.question.shown, "the card came down");
    assert!(host.submitter.interval().is_some(), "the request is out");
    assert_eq!(answers_of(&mut host), 1, "and its answer came back");
    let first = served.join().expect("the listener answered");
    assert!(first.starts_with("POST /submit "), "was {first:?}");
}

/// Escape on the card sends nothing.
#[test]
fn a_form_answered_no_is_not_sent() {
    let mut host = a_host();
    host.event(submits_to("http://127.0.0.1:9/submit"));
    assert!(host.question.shown, "the question is up");
    host.press(&Key::Named(NamedKey::Escape));
    assert!(!host.question.shown, "the card came down");
    assert!(host.submitter.interval().is_none(), "nothing went out");
}

/// At `refuse` nothing is asked and nothing is sent; at `send` nothing is asked and the
/// request goes out — the two levels that need no card, the first of them the control.
#[test]
fn a_form_at_refuse_is_not_sent_and_at_send_is_not_asked() {
    let mut host = a_host();
    host.submissions = viewer_host::Submissions::Refuse;
    host.event(submits_to("http://127.0.0.1:9/submit"));
    assert!(!host.question.shown, "no question at refuse");
    assert!(host.submitter.interval().is_none(), "and nothing sent");

    let (url, served) = one_answer();
    host.submissions = viewer_host::Submissions::Send;
    host.event(submits_to(&url));
    assert!(!host.question.shown, "no question at send");
    assert_eq!(answers_of(&mut host), 1, "sent, and answered");
    assert!(
        served
            .join()
            .expect("answered")
            .starts_with("POST /submit ")
    );
}

/// An import named by an absolute URI is asked at the submissions level and fetched on a
/// `yes`, as in the other three windows (ADRs 1527, 2076).
#[test]
fn an_import_named_by_a_uri_is_asked_and_fetched_on_a_yes() {
    let (url, served) = one_answer();
    let mut host = a_host();
    host.event(Event::NeedsFile {
        document: DocumentId(0),
        purpose: viewer_core::Purpose::ImportData,
        name: url.clone(),
        beside: false,
    });
    assert!(
        matches!(&host.asked, Some(super::asking::Asked::Fetch { url: asked, .. }) if *asked == url),
        "the fetch is the question up"
    );
    host.press(&Key::Named(NamedKey::Enter));
    assert_eq!(answers_of(&mut host), 1, "fetched, and answered");
    assert!(served.join().expect("answered").starts_with("GET /submit "));
}

/// A refusal that leaves the worker alive is not a death: nothing is restarted and the window
/// reports it, which is the old behaviour and must stay the old behaviour.
#[test]
fn a_refusal_the_worker_survived_is_not_restarted_from() {
    let mut host = a_host();
    host.died(&ConfinedError::Refused {
        detail: "a raster in a second pixel layout".to_owned(),
    });
    assert!(host.resume.is_none(), "nothing to start again");
    assert!(host.stopped.is_some(), "and it is reported");
}

/// The reader's own abort is never restarted from: a `Cancelled` viewer is one somebody
/// ended on purpose (ADR 0241), and a second worker would undo the key press.
#[test]
fn the_readers_abort_is_not_undone_by_a_restart() {
    let mut host = a_host();
    host.died(&ConfinedError::Cancelled);
    assert!(host.resume.is_none(), "Escape meant Escape");
    assert!(host.stopped.is_some());
}

/// A restart cannot fire once the window has stopped for another reason — the reader aborted
/// between the death and the loop's next turn, which is a race a deferred resume can lose.
#[test]
fn an_abort_between_the_death_and_the_restart_wins() {
    let mut host = a_host();
    host.died(&ConfinedError::WorkerDied {
        detail: "killed by signal 6".to_owned(),
    });
    let owed = host.resume.take().expect("a restart is owed");
    host.abort();
    host.reopen(owed);
    assert!(
        host.confined.is_none(),
        "no worker was started for a window the reader had ended"
    );
    assert!(
        host.stopped
            .as_deref()
            .is_some_and(|said| said.contains("aborted")),
        "and the abort is still what the window says, was {:?}",
        host.stopped
    );
}
