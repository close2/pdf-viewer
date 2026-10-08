//! ISO 32000-2 §12.8.3.4.4's signature policy, fetched from the URL the signature names, checked
//! where it lands: a listener on this machine's loopback serving the policy document.
//!
//! §12.8.3.4.4 makes ETSI EN 319 122-1 clause 5.2.9's rules apply to a `PAdES-E-EPES` signature,
//! and clause 5.2.9.2's URL qualifier says where a copy of the policy document can be obtained.
//! `viewer_host::policy::may_fetch_signature_policy` asks the reader's network level,
//! `fetch_signature_policy` fetches through the submit client, and `pdf_signature`'s comparison
//! decides whether the copy is the one the signer committed to (ADR 1728). What these hold is the
//! three outcomes the reader is told — bound and shown, a digest that does not match, not fetched
//! at this level — the refusals that come before the level, and that every outcome says the
//! policy's constraints were not enforced.
//!
//! The server is a `std::net::TcpListener` bound to `127.0.0.1:0` and read by hand, as
//! `fetch_import.rs`'s is, so no dependency is taken to test the one that was.

#![expect(
    clippy::expect_used,
    reason = "a test's failure is its purpose, and these helpers run outside #[test] bodies \
              where `allow-panic-in-tests` does not reach"
)]

use std::io::{BufRead as _, BufReader, Write as _};
use std::net::TcpListener;

use pdf_signature::cms::Digest;
use pdf_signature::policy::{Commitment, PublishedPolicy, Specification};
use viewer_host::policy::{
    fetch_signature_policy, may_fetch_signature_policy, signature_policy_declined,
};
use viewer_host::submit::Reply;
use viewer_host::{Sending, Submissions};

/// The policy document the loopback serves: a one-page PDF, as the policy the corpus's thirteen
/// Spanish signatures name is (ADR 1709).
const POLICY: &[u8] = b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n\
                        2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n\
                        trailer\n<< /Root 1 0 R >>\n%%EOF\n";

/// The identifier the corpus's thirteen signatures sign under, and SHA-1, which is theirs too.
const IDENTIFIER: &str = "2.16.724.1.3.1.1.2.1.9";

/// What a signature under a published policy hands a host: the URL its qualifier names, and the
/// SHA-1 of [`POLICY`] as the digest the signer committed to. No specification is named, which
/// is every explicit policy on this disk.
fn published(url: &str) -> PublishedPolicy {
    PublishedPolicy {
        identifier: IDENTIFIER.to_owned(),
        url: url.to_owned(),
        commitment: Commitment::Stated {
            digest: Digest::Sha1,
            value: Digest::Sha1.compute(&[POLICY]),
        },
        specification: None,
    }
}

/// A listener that answers one request with `status`, `content_type` and `body`.
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

/// The sentence every outcome ends with, for a signature naming no specification.
const UNENFORCED: &str = "§12.8.3.4.4 also requires a signature handler to enforce the policy's \
                          constraints, and this program does not: the signature names no \
                          specification for the policy's syntax";

/// The whole path at `send`: the level lets it through, the GET reaches the server, the copy
/// hashes to the signed digest, and it comes back as a document to open beside this one.
#[test]
fn a_policy_document_whose_copy_hashes_to_the_signed_digest_is_bound_and_shown() {
    let (base, server) = serve_once("200 OK", "application/pdf", POLICY.to_vec());
    let policy = published(&format!("{base}/politica_de_firma.pdf"));
    assert_eq!(
        may_fetch_signature_policy(&policy, Submissions::Send),
        Sending::Send
    );
    let Reply::Document { bytes, note } = fetch_signature_policy(&policy, None) else {
        panic!("a bound PDF is opened beside the document");
    };
    assert_eq!(
        server.join().expect("the server ran"),
        "GET /politica_de_firma.pdf HTTP/1.1"
    );
    assert_eq!(bytes, POLICY);
    assert!(
        note.starts_with(&format!("signature policy {IDENTIFIER}: ")),
        "{note}"
    );
    assert!(
        note.contains("is the document the signer committed to: it hashes to the SHA1 digest"),
        "{note}"
    );
    assert!(note.contains(UNENFORCED), "{note}");
}

/// The control for the test above (trap 13): the same exchange with one octet of the served copy
/// turned over. Nothing is opened, and the sentence says why without calling it a different policy.
#[test]
fn a_copy_that_differs_by_one_octet_is_said_and_not_shown() {
    let mut altered = POLICY.to_vec();
    altered[1] ^= 0x01;
    let (base, server) = serve_once("200 OK", "application/pdf", altered);
    let policy = published(&format!("{base}/politica_de_firma.pdf"));
    let Reply::Say(note) = fetch_signature_policy(&policy, None) else {
        panic!("a copy that does not hash to the signed digest is not opened");
    };
    let _ = server.join();
    assert!(
        note.contains("does not hash to the SHA1 digest the signer signed over it"),
        "{note}"
    );
    assert!(
        note.contains("this signature does not name and leaves to its context"),
        "{note}"
    );
    assert!(note.contains(UNENFORCED), "{note}");
}

/// Not fetched at this level: the four levels' answers, and the sentence a refusal becomes.
#[test]
fn the_reader_s_network_level_decides_whether_the_policy_is_fetched() {
    let policy = published("https://administracionelectronica.gob.es/politica_de_firma.pdf");
    let Sending::Refuse(why) = may_fetch_signature_policy(&policy, Submissions::Refuse) else {
        panic!("refuse fetches nothing");
    };
    let declined = signature_policy_declined(&policy, &why);
    assert!(
        declined.contains("was not fetched — this reader is set to send nothing"),
        "{declined}"
    );
    assert!(declined.contains(UNENFORCED), "{declined}");
    let Sending::Ask(question) = may_fetch_signature_policy(&policy, Submissions::Ask) else {
        panic!("ask puts the request to the person");
    };
    assert!(question.reasons.contains(IDENTIFIER), "{question:?}");
    assert!(question.reasons.contains(&policy.url), "{question:?}");
    let Sending::Warn(_) = may_fetch_signature_policy(&policy, Submissions::Warn) else {
        panic!("warn fetches and says so");
    };
    assert_eq!(
        may_fetch_signature_policy(&policy, Submissions::Send),
        Sending::Send
    );
}

/// What a document cannot reach at any level: a scheme this reader does not fetch from, and a URL
/// whose host fails the check made before the TLS stack.
#[test]
fn a_policy_url_outside_the_fetched_schemes_is_refused_at_every_level() {
    for url in [
        "file:///etc/passwd",
        "ftp://example.org/policy.pdf",
        "https://user@example.org/policy.pdf",
    ] {
        let policy = published(url);
        for level in Submissions::ALL {
            assert!(
                matches!(
                    may_fetch_signature_policy(&policy, level),
                    Sending::Refuse(_)
                ),
                "{url} at {level:?}"
            );
        }
    }
}

/// A server that answers anything but a 2xx sent back no copy, and nothing is compared.
#[test]
fn a_server_that_answers_not_found_returns_no_copy() {
    let (base, server) = serve_once("404 Not Found", "text/html", b"<p>gone</p>".to_vec());
    let policy = published(&format!("{base}/politica_de_firma.pdf"));
    let Reply::Say(note) = fetch_signature_policy(&policy, None) else {
        panic!("a 404 opens nothing");
    };
    let _ = server.join();
    assert!(note.contains("answered 404"), "{note}");
    assert!(
        note.contains("so no copy of the policy's document came back"),
        "{note}"
    );
}

/// A signature whose stated specification is named in the closing sentence rather than the
/// context: the one shape no signature on this disk has, built so the other arm is not dead.
#[test]
fn a_named_specification_is_said_where_the_constraints_are_not_enforced() {
    let mut policy = published("ftp://example.org/policy.pdf");
    policy.specification = Some(Specification::ObjectIdentifier("1.2.3.6".to_owned()));
    let declined = signature_policy_declined(&policy, "the scheme is not fetched");
    assert!(
        declined.contains(
            "they are written under the specification identified by 1.2.3.6, a specification \
             it does not hold"
        ),
        "{declined}"
    );
}

/// The window's path rather than the function's: `Submitter::fetch_policy` puts the GET on a
/// thread of its own, the answer is collected as a submission's is, and what it comes to is the
/// same [`Reply`] the blocking function gives — a bound PDF to open beside the document whose
/// signature named it, for that document (ADR 1738).
#[test]
fn a_window_s_fetch_of_a_policy_is_collected_as_its_bound_copy() {
    let (base, server) = serve_once("200 OK", "application/pdf", POLICY.to_vec());
    let policy = published(&format!("{base}/politica_de_firma.pdf"));
    let mut submitter = viewer_host::submit::Submitter::new();
    submitter
        .fetch_policy(viewer_core::DocumentId(7), policy, None, None)
        .expect("a thread to fetch on");
    assert_eq!(
        submitter.interval(),
        Some(viewer_host::submit::LOOK),
        "an answer is outstanding, so the window's timer runs"
    );
    let mut returned = Vec::new();
    for _ in 0..500 {
        returned.extend(submitter.collect());
        if !returned.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let _ = server.join();
    let [returned] = <[_; 1]>::try_from(returned).expect("one answer arrives");
    assert_eq!(returned.document, viewer_core::DocumentId(7));
    assert_eq!(
        submitter.interval(),
        None,
        "and nothing is outstanding after it"
    );
    let Reply::Document { bytes, note } = returned.reply() else {
        panic!("a bound PDF is opened beside the document");
    };
    assert_eq!(bytes, POLICY);
    assert!(note.contains(UNENFORCED), "{note}");
}

/// The control (trap 13): the same window path to a server that answers 404 says so and opens
/// nothing, so the test above cannot pass on a reply that ignored the server.
#[test]
fn a_window_s_fetch_of_a_policy_that_is_not_found_is_said() {
    let (base, server) = serve_once("404 Not Found", "text/html", b"<p>no</p>".to_vec());
    let policy = published(&format!("{base}/politica_de_firma.pdf"));
    let mut submitter = viewer_host::submit::Submitter::new();
    submitter
        .fetch_policy(viewer_core::DocumentId(7), policy, None, None)
        .expect("a thread to fetch on");
    let mut returned = Vec::new();
    for _ in 0..500 {
        returned.extend(submitter.collect());
        if !returned.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let _ = server.join();
    let [returned] = <[_; 1]>::try_from(returned).expect("one answer arrives");
    let Reply::Say(note) = returned.reply() else {
        panic!("a 404 is said, not opened");
    };
    assert!(note.contains("answered 404"), "{note}");
}
