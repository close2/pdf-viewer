//! ISO 32000-2 §12.7.6.2's verb, checked where it lands: a listener on this machine's loopback.
//!
//! "Upon invocation of a submit-form action, an interactive PDF processor shall transmit the names
//! and values of selected interactive form fields to a specified uniform resource locator (URL)."
//! `pdf_model::submission` has its own tests for *which* names and values; what these hold is that
//! what it composed is what arrives — the method Table 240 bit 4 chose, the media type the format
//! and §12.7.5.3 chose, and the body byte for byte — and that the policy in front of the network
//! answers in the order ADR 1291 fixes.
//!
//! The server is a `std::net::TcpListener` bound to `127.0.0.1:0` and read by hand, so no
//! dependency is taken to test the one that was (ADR 1291).

#![expect(
    clippy::expect_used,
    clippy::panic,
    reason = "a test's failure is its purpose, and these helpers run outside #[test] bodies \
              where `allow-panic-in-tests` does not reach"
)]

use std::fmt::Write as _;
use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::TcpListener;
use std::time::{Duration, Instant};

use pdf_model::action::{Action, DataFormat, SubmitForm};
use pdf_model::submission::{Method, Submission, compose};
use pdf_model::view::ViewState;
use pdf_syntax::{Document, Object, ObjectId};
use viewer_core::{Command, DocumentId, Event, Viewer};
use viewer_host::submit::{Reply, Submitter, TransmitError, transmit};
use viewer_host::{Restrictions, Row, Sending, Submissions, may_submit};

/// A form of two fields with values and one without, beside §7.5.4's table.
fn form() -> Vec<u8> {
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [4 0 R 5 0 R] >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Annots [4 0 R 5 0 R] >>",
        "<< /Type /Annot /Subtype /Widget /Rect [10 100 190 130] /F 4 /FT /Tx /T (name) \
         /V (Ada) >>",
        "<< /Type /Annot /Subtype /Widget /Rect [10 50 190 80] /F 4 /FT /Tx /T (city) \
         /V (London) >>",
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
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<0102> <0304>] >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.into_bytes()
}

/// Table 239's action with these flags, sending to `url`.
fn action(url: &str, flags: u32) -> SubmitForm {
    let source = format!(
        "%PDF-2.0\n1 0 obj\n<< /Type /Catalog >>\nendobj\n\
         2 0 obj\n<< /S /SubmitForm /F << /FS /URL /F ({url}) >> /Flags {flags} >>\nendobj\n\
         trailer\n<< /Root 1 0 R /Size 3 >>\n"
    );
    let holder = Document::open(pdf_syntax::FileBytes::from(source.into_bytes()))
        .expect("the action parses");
    match pdf_model::action::read(&holder, &Object::Reference(ObjectId::new(2, 0)))
        .into_iter()
        .next()
    {
        Some(Action::SubmitForm(submit)) => submit,
        other => panic!("Table 239's action is read whole: {other:?}"),
    }
}

/// What `compose` makes of the form under that action.
fn composed(url: &str, flags: u32) -> Submission {
    let document = Document::open(pdf_syntax::FileBytes::from(form())).expect("the form parses");
    let view = ViewState::of(&document);
    compose(&document, &view, &action(url, flags), None).expect("the composition succeeds")
}

/// One request as a server reads it: the request line, the headers, the body.
#[derive(Debug)]
struct Arrived {
    /// `POST` or `GET`.
    method: String,
    /// The request target — the path and the query.
    target: String,
    /// `Content-Type`, where one was sent.
    media_type: Option<String>,
    /// The entity body, `Content-Length` bytes of it.
    body: Vec<u8>,
}

/// A listener that answers one request with `answer` and hands back what it was sent.
fn serve_once(
    content_type: &'static str,
    answer: Vec<u8>,
) -> (String, std::thread::JoinHandle<Arrived>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("the loopback takes a listener");
    let port = listener.local_addr().expect("a bound port").port();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("one connection arrives");
        let mut reader = BufReader::new(stream.try_clone().expect("the stream clones"));
        let mut line = String::new();
        reader.read_line(&mut line).expect("a request line");
        let mut words = line.split_whitespace();
        let method = words.next().unwrap_or_default().to_owned();
        let target = words.next().unwrap_or_default().to_owned();
        let (mut length, mut media_type) = (0_usize, None);
        loop {
            let mut header = String::new();
            reader.read_line(&mut header).expect("a header line");
            let header = header.trim_end();
            if header.is_empty() {
                break;
            }
            if let Some((name, value)) = header.split_once(':') {
                match name.to_ascii_lowercase().as_str() {
                    "content-length" => length = value.trim().parse().unwrap_or(0),
                    "content-type" => media_type = Some(value.trim().to_owned()),
                    _ => {}
                }
            }
        }
        let mut body = vec![0; length];
        reader
            .read_exact(&mut body)
            .expect("the body the length names");
        let mut stream = stream;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\
             Connection: close\r\n\r\n",
            answer.len()
        )
        .expect("the answer's head is written");
        stream
            .write_all(&answer)
            .expect("the answer's body is written");
        Arrived {
            method,
            target,
            media_type,
            body,
        }
    });
    (format!("http://127.0.0.1:{port}/cgi"), server)
}

/// Every format §12.7.6.2 names, and the one method bit 4 can change, arrives as it was composed.
///
/// Table 240: bit 3 set is "HTML Form format", clear is FDF; bit 4 set is "an HTTP GET request";
/// bit 6 is XFDF; bit 9 is "the document … as PDF, using the MIME media type application/pdf".
/// And §12.7.8.1: "FDF shall use the MIME media type application/vnd.fdf".
#[test]
fn each_format_reaches_the_server_as_it_was_composed() {
    let cases: [(u32, &str, Option<&str>); 5] = [
        (0, "POST", Some("application/vnd.fdf")),
        (1 << 2, "POST", Some("application/x-www-form-urlencoded")),
        ((1 << 2) | (1 << 3), "GET", None),
        (1 << 5, "POST", Some("application/xfdf")),
        (1 << 8, "POST", Some("application/pdf")),
    ];
    for (flags, method, media_type) in cases {
        let (url, server) = serve_once("text/plain", b"received".to_vec());
        let submission = composed(&url, flags);
        let answer = transmit(&submission).expect("the loopback answers");
        let arrived = server.join().expect("the server thread returns");

        assert_eq!(arrived.method, method, "flags {flags}");
        assert_eq!(arrived.media_type.as_deref(), media_type, "flags {flags}");
        assert_eq!(
            arrived.body, submission.body,
            "flags {flags}: the body, byte for byte"
        );
        assert_eq!(answer.status, 200);
        assert_eq!(answer.media_type.as_deref(), Some("text/plain"));
        assert_eq!(answer.body, b"received");
        match submission.method {
            // A GET carries its data in the query and nothing in the body.
            Method::Get => {
                let query = submission.url.split_once('?').map(|(_, query)| query);
                assert_eq!(
                    arrived.target,
                    format!("/cgi?{}", query.expect("a GET's data is its query")),
                    "flags {flags}"
                );
                assert!(arrived.body.is_empty());
            }
            Method::Post => assert_eq!(arrived.target, "/cgi", "flags {flags}"),
        }
    }
    // The three bodies with a shape of their own, checked for it rather than for this tree's
    // spelling of it: §12.7.8.1's header, HTML 4.01 section 17.13.4's pairs, and a whole PDF.
    let fdf = composed("http://127.0.0.1:1/cgi", 0);
    assert!(fdf.body.starts_with(b"%FDF-"));
    let html = composed("http://127.0.0.1:1/cgi", 1 << 2);
    let pairs = String::from_utf8_lossy(&html.body);
    let mut pairs: Vec<&str> = pairs.split('&').collect();
    pairs.sort_unstable();
    assert_eq!(pairs, ["city=London", "name=Ada"]);
    let pdf = composed("http://127.0.0.1:1/cgi", 1 << 8);
    assert!(pdf.body.starts_with(b"%PDF-"));
}

/// A `file` or `mailto` URL is refused at every level alike — which is what "before the level is
/// read" means from outside — and the client refuses it again on its own.
#[test]
fn a_scheme_outside_http_is_refused_before_the_level_is_read() {
    for url in [
        "file:///etc/passwd",
        "mailto:forms@example.invalid",
        "FILE:///tmp/x",
    ] {
        let submission = composed(url, 1 << 2);
        let answers: Vec<Sending> = Submissions::ALL
            .into_iter()
            .map(|level| may_submit(&submission, level))
            .collect();
        let Sending::Refuse(first) = &answers[0] else {
            panic!("{url}: refused at `refuse`: {answers:?}");
        };
        assert!(
            first.contains("is not one of the schemes"),
            "{url}: {first}"
        );
        for answer in &answers {
            assert_eq!(answer, &answers[0], "{url}: the level decided nothing");
        }
        assert!(matches!(
            transmit(&submission),
            Err(TransmitError::Scheme(_))
        ));
    }
    // And the four levels answer four ways once the scheme is one a form is sent to.
    let submission = composed("https://example.invalid/cgi", 1 << 2);
    assert!(matches!(
        may_submit(&submission, Submissions::Refuse),
        Sending::Refuse(_)
    ));
    assert!(matches!(
        may_submit(&submission, Submissions::Ask),
        Sending::Ask(_)
    ));
    assert!(matches!(
        may_submit(&submission, Submissions::Warn),
        Sending::Warn(_)
    ));
    assert_eq!(may_submit(&submission, Submissions::Send), Sending::Send);
}

/// The owner's default (`doc/questions/Q98`): *ask*, in the type, in a window's menu state, and
/// in what the policy answers for a window nobody has configured.
#[test]
fn the_default_level_is_ask() {
    assert_eq!(Submissions::default(), Submissions::Ask);
    let restrictions = Restrictions::new(viewer_core::RestrictionPolicy::default());
    assert_eq!(restrictions.submissions(), Submissions::Ask);
    let submission = composed("http://127.0.0.1:1/cgi", 0);
    let Sending::Ask(question) = may_submit(&submission, restrictions.submissions()) else {
        panic!("a window at its defaults asks");
    };
    assert!(question.reasons.contains("http://127.0.0.1:1/cgi"));
    assert!(question.reasons.contains("application/vnd.fdf"));
}

/// The restriction menu's third group: one act, four levels, `ask` ticked, and a pick moves the
/// tick without sending the viewer anything.
#[test]
fn the_menu_holds_the_level_and_a_pick_moves_the_tick() {
    let mut restrictions = Restrictions::new(viewer_core::RestrictionPolicy::default());
    let ticked = |restrictions: &Restrictions| -> Vec<(&'static str, bool)> {
        restrictions
            .rows()
            .into_iter()
            .filter_map(|row| match row {
                Row::Sending(entry) => Some((entry.label, entry.chosen)),
                _ => None,
            })
            .collect()
    };
    assert_eq!(
        ticked(&restrictions),
        [
            ("refuse", false),
            ("ask", true),
            ("warn", false),
            ("send", false)
        ]
    );
    assert!(
        restrictions
            .rows()
            .iter()
            .any(|row| matches!(row, Row::Machine { .. }))
    );
    restrictions.send(Submissions::Send);
    assert_eq!(restrictions.submissions(), Submissions::Send);
    assert_eq!(ticked(&restrictions)[3], ("send", true));
}

/// The whole path a window takes: sent on its own thread, collected, and an FDF answer imported
/// into the document that sent the form, with Table 246's `/Status` displayed.
///
/// Table 246: `/Status` is "[a] status string that shall be displayed indicating the result of an
/// action, typically a submit-form action".
#[test]
fn an_fdf_answer_is_imported_into_the_document_that_sent_the_form() {
    let answer = b"%FDF-1.2\n1 0 obj\n<< /FDF << /Fields [<< /T (name) /V (Grace) >>] \
                   /Status (Thank you) >> >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n"
        .to_vec();
    let (url, server) = serve_once("application/vnd.fdf", answer);
    let document = DocumentId(7);
    let mut viewer = Viewer::new(400, 400, 1.0);
    viewer
        .handle(Command::Open {
            id: document,
            bytes: form().into(),
            password: None,
            fragment: None,
        })
        .for_each(drop);

    let mut submitter = Submitter::new();
    submitter
        .send(document, composed(&url, 1 << 2), None, None)
        .expect("a thread to send on");
    assert!(
        submitter.interval().is_some(),
        "a window's timer runs while one is out"
    );
    let started = Instant::now();
    let returned = loop {
        if let Some(returned) = submitter.collect().pop() {
            break returned;
        }
        assert!(
            started.elapsed() < Duration::from_secs(30),
            "the loopback answers"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(submitter.interval().is_none(), "and stops once it is in");
    let _ = server.join().expect("the server thread returns");

    assert_eq!(returned.document, document);
    let source = returned.url.clone();
    let Reply::Import {
        format,
        bytes,
        note,
    } = returned.reply()
    else {
        panic!("an FDF answer is an import");
    };
    assert_eq!(format, DataFormat::Fdf);
    assert!(note.contains("answered 200"), "{note}");

    let notes: Vec<String> = viewer
        .handle(Command::Respond {
            document,
            source,
            format,
            bytes,
        })
        .filter_map(|event| match event {
            Event::Reported { notes, .. } => Some(notes),
            _ => None,
        })
        .flatten()
        .collect();
    assert!(
        notes
            .iter()
            .any(|note| note == "submit-form answer: status — Thank you"),
        "{notes:?}"
    );
    assert!(
        notes.iter().any(|note| note
            .starts_with("submit-form answer: 1 field(s) imported from http://127.0.0.1:")),
        "{notes:?}"
    );
}
