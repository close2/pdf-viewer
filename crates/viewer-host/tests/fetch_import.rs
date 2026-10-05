//! ISO 32000-2 Table Annex O.4's `fdf` naming an absolute URI, checked where it lands: a listener on
//! this machine's loopback.
//!
//! Table Annex O.4: "Open the document and then import the data from the specified FDF or XFDF
//! file. The URI shall be either a relative or absolute URI to an FDF or XFDF file." A relative one
//! is a file beside the document (`fragments.rs` in `viewer-ui`); an absolute one is a GET over the
//! network, asked under the `Submissions` level by `viewer_host::may_fetch_import`, fetched by the
//! submit client, and imported into the document that asked (ADR 1527). What these hold is the
//! level's four answers, the refusals that come before it, the bound on the answer, and that a
//! body which is not form data is said rather than dropped.
//!
//! The server is a `std::net::TcpListener` bound to `127.0.0.1:0` and read by hand, so no
//! dependency is taken to test the one that was (ADR 1291, `submit.rs`'s construction).

#![expect(
    clippy::expect_used,
    reason = "a test's failure is its purpose, and these helpers run outside #[test] bodies \
              where `allow-panic-in-tests` does not reach"
)]

use std::fmt::Write as _;
use std::io::{BufRead as _, BufReader, Write as _};
use std::net::TcpListener;
use std::time::{Duration, Instant};

use pdf_model::action::DataFormat;
use viewer_core::{Answered, Command, DocumentId, Event, Purpose, Viewer};
use viewer_host::submit::{RESPONSE_LIMIT, Reply, Returned, Submitter};
use viewer_host::{Sending, Submissions, import_is_fetched, may_fetch_import};

/// A form of one text field, `name`, valued `Ada`, beside §7.5.4's table.
fn form() -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R] >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Annots [4 0 R] >>",
        "<< /Type /Annot /Subtype /Widget /Rect [10 100 190 130] /F 4 /FT /Tx /T (name) \
         /V (Ada) >>",
    ];
    let size = objects.len().saturating_add(1);
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (number, object) in (1_usize..).zip(objects) {
        offsets.push(out.len());
        let _ = write!(out, "{number} 0 obj\n{object}\nendobj\n");
    }
    let xref = out.len();
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

/// §12.7.8's form data setting `name` to `Grace`, with Table 246's `/Status`.
const FDF: &[u8] = b"%FDF-1.2\n1 0 obj\n<< /FDF << /Fields [<< /T (name) /V (Grace) >>] \
                     /Status (Fetched) >> >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n";

/// A listener that answers one request with `status`, `content_type` and `body`, and hands back
/// the request line it read. The body is written in full or until the client stops reading, which
/// is what a client enforcing a bound does.
fn serve_once(
    status: &'static str,
    content_type: &'static str,
    body: Vec<u8>,
) -> (String, std::thread::JoinHandle<String>) {
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
        // A client that refused the answer part-way closes the connection, and the write that
        // fails is that refusal arriving rather than a fault in the test.
        let _ = write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\
             Connection: close\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(&body);
        line.trim_end().to_owned()
    });
    (format!("http://127.0.0.1:{port}"), server)
}

/// A viewer holding [`form`] as `document`, opened under `fragment`, and the events it answered.
fn opened(document: DocumentId, fragment: &str) -> (Viewer, Vec<Event>) {
    let mut viewer = Viewer::new(400, 400, 1.0);
    let events = viewer
        .handle(Command::Open {
            id: document,
            bytes: form().into(),
            password: None,
            fragment: Some(fragment.to_owned()),
        })
        .collect();
    (viewer, events)
}

/// Every sentence a run of events reported.
fn notes(events: impl IntoIterator<Item = Event>) -> Vec<String> {
    events
        .into_iter()
        .filter_map(|event| match event {
            Event::Reported { notes, .. } => Some(notes),
            _ => None,
        })
        .flatten()
        .collect()
}

/// One fetch on the submitter's thread, waited for.
fn fetched(document: DocumentId, url: &str) -> Returned {
    let mut submitter = Submitter::new();
    submitter
        .fetch(document, url.to_owned(), None, None)
        .expect("a thread to fetch on");
    assert!(
        submitter.interval().is_some(),
        "a window's timer runs while one is out"
    );
    let started = Instant::now();
    loop {
        if let Some(returned) = submitter.collect().pop() {
            assert!(submitter.interval().is_none(), "and stops once it is in");
            return returned;
        }
        assert!(
            started.elapsed() < Duration::from_mins(1),
            "the loopback answers"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// The whole path a window takes at `send`: the fragment's URI crosses as `Event::NeedsFile`, the
/// policy lets it through, the submitter's GET reaches the server, and the answer is imported into
/// the document that asked — with the sentence the import says whichever route its file took.
#[test]
fn an_fdf_on_a_server_is_fetched_and_imported_into_the_document_that_asked() {
    let (base, server) = serve_once("200 OK", "application/octet-stream", FDF.to_vec());
    let url = format!("{base}/values.fdf");
    let document = DocumentId(3);
    let (mut viewer, events) = opened(document, &format!("fdf={url}"));
    let asked = events.iter().find_map(|event| match event {
        Event::NeedsFile {
            document: asking,
            purpose: Purpose::ImportData,
            name,
            ..
        } if *asking == document => Some(name.clone()),
        _ => None,
    });
    let Some(name) = asked else {
        panic!("the fragment asked the host for its file: {events:?}");
    };
    assert_eq!(name, url);
    assert!(import_is_fetched(&name), "an absolute URI is fetched");
    assert_eq!(may_fetch_import(&name, Submissions::Send), Sending::Send);

    let returned = fetched(document, &name);
    assert_eq!(
        server.join().expect("the server thread returns"),
        "GET /values.fdf HTTP/1.1"
    );
    assert_eq!(returned.document, document);
    let Reply::Import {
        answers,
        format,
        bytes,
        note,
    } = returned.reply()
    else {
        panic!("a 200 answer to an .fdf name is an import");
    };
    // The name decides the format, not the server's `application/octet-stream`.
    assert_eq!((answers, format), (Answered::Import, DataFormat::Fdf));
    assert!(note.contains("answered 200"), "{note}");

    let said = notes(viewer.handle(Command::Respond {
        document,
        answers,
        source: url.clone(),
        format,
        bytes,
    }));
    assert!(
        said.contains(&"import-data: status — Fetched".to_owned()),
        "{said:?}"
    );
    assert!(
        said.contains(&format!(
            "import-data: 1 field(s) from {url}, into 1 widget(s)"
        )),
        "{said:?}"
    );
}

/// A relative name stays beside the document, and an absolute one goes to the network policy.
#[test]
fn only_an_absolute_uri_is_fetched() {
    assert!(!import_is_fetched("values.fdf"));
    assert!(!import_is_fetched("data/values.xfdf"));
    assert!(import_is_fetched("http://127.0.0.1/values.fdf"));
    assert!(import_is_fetched("file:///etc/values.fdf"));
}

/// `CLAUDE.md` principle 3's four levels, each answered, and `refuse` without a connection.
#[test]
fn each_level_answers_and_refuse_fetches_nothing() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("the loopback takes a listener");
    listener
        .set_nonblocking(true)
        .expect("a listener that can be asked without waiting");
    let port = listener.local_addr().expect("a bound port").port();
    let url = format!("http://127.0.0.1:{port}/values.fdf");

    let Sending::Refuse(why) = may_fetch_import(&url, Submissions::Refuse) else {
        panic!("refuse refuses");
    };
    assert!(why.contains(Submissions::Refuse.as_str()), "{why}");
    assert!(
        viewer_host::fetch_note(&url, Some(&why)).starts_with("import-data: declined — "),
        "the refusal is said as the import's"
    );
    let Sending::Ask(question) = may_fetch_import(&url, Submissions::Ask) else {
        panic!("ask asks");
    };
    assert!(question.reasons.contains(&url), "{}", question.reasons);
    let Sending::Warn(warned) = may_fetch_import(&url, Submissions::Warn) else {
        panic!("warn fetches and says so");
    };
    assert!(warned.contains("without asking"), "{warned}");
    assert_eq!(may_fetch_import(&url, Submissions::Send), Sending::Send);
    assert_eq!(Submissions::default(), Submissions::Ask);

    // Nothing above reached the network: a policy answers, and only a window fetches.
    assert!(
        listener.accept().is_err(),
        "no level's answer connected to the server"
    );
}

/// A scheme outside `SUBMIT_SCHEMES`, and a URL `check_url` refuses, are refused at every level —
/// `send` included, because neither is a thing a reader's level turns on.
#[test]
fn a_scheme_this_reader_does_not_fetch_from_is_refused_at_every_level() {
    for url in [
        "file:///etc/values.fdf",
        "ftp://127.0.0.1/values.fdf",
        "http://user@127.0.0.1/values.fdf",
    ] {
        for level in Submissions::ALL {
            let Sending::Refuse(why) = may_fetch_import(url, level) else {
                panic!("{url} is refused at {}", level.as_str());
            };
            assert!(
                why.contains("is not one of the schemes") || why.contains("is not sent"),
                "{url}: {why}"
            );
        }
    }
}

/// An answer over `RESPONSE_LIMIT` is refused by name, not truncated and imported.
#[test]
fn an_answer_over_the_bound_is_refused_by_name() {
    let over = usize::try_from(RESPONSE_LIMIT).expect("the bound fits in memory") + 1;
    let (base, server) = serve_once("200 OK", "application/vnd.fdf", vec![b' '; over]);
    let url = format!("{base}/values.fdf");
    let returned = fetched(DocumentId(1), &url);
    let _ = server.join().expect("the server thread returns");
    let Reply::Say(said) = returned.reply() else {
        panic!("an answer over the bound is no import");
    };
    assert!(
        said.starts_with(&format!("import-data: {url} was not fetched — ")),
        "{said}"
    );
}

/// A body that is not the FDF its name says is reported by the import that reads it.
#[test]
fn a_body_that_is_not_form_data_is_reported() {
    let (base, server) = serve_once("200 OK", "text/html", b"<html>no form here</html>".to_vec());
    let url = format!("{base}/values.fdf");
    let document = DocumentId(2);
    let (mut viewer, _) = opened(document, &format!("fdf={url}"));
    let returned = fetched(document, &url);
    let _ = server.join().expect("the server thread returns");
    let Reply::Import {
        answers,
        format,
        bytes,
        ..
    } = returned.reply()
    else {
        panic!("a 200 answer to an .fdf name is handed to the import");
    };
    let said = notes(viewer.handle(Command::Respond {
        document,
        answers,
        source: url.clone(),
        format,
        bytes,
    }));
    assert!(
        said.iter()
            .any(|note| note.starts_with(&format!("import-data: cannot read {url}: "))),
        "{said:?}"
    );
}

/// A status outside 2xx imports nothing and says what the server answered.
#[test]
fn an_error_status_imports_nothing() {
    let (base, server) = serve_once("404 Not Found", "application/vnd.fdf", FDF.to_vec());
    let url = format!("{base}/values.fdf");
    let returned = fetched(DocumentId(1), &url);
    let _ = server.join().expect("the server thread returns");
    let Reply::Say(said) = returned.reply() else {
        panic!("a 404 is no import");
    };
    assert!(said.contains("answered 404"), "{said}");
    assert!(said.ends_with("nothing was imported"), "{said}");
}
