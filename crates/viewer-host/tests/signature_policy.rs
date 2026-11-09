//! ISO 32000-2 §12.8.3.4.4's signature policy, fetched from the URL the signature names, checked
//! where it lands: a listener on this machine's loopback serving the policy document.
//!
//! §12.8.3.4.4 makes ETSI EN 319 122-1 clause 5.2.9's rules apply to a `PAdES-E-EPES` signature,
//! and clause 5.2.9.2's URL qualifier says where a copy of the policy document can be obtained.
//! `viewer_host::policy::may_fetch_signature_policy` asks the reader's network level,
//! `fetch_signature_policy` fetches through the submit client, and `pdf_signature`'s comparison
//! decides whether the copy is the one the signer committed to (ADR 1728). What these hold is the
//! three outcomes the reader is told — bound and shown, a digest that does not match, not fetched
//! at this level — the refusals that come before the level, and where each outcome says the
//! policy's constraints are enforced from. The last three hold the finish: a copy the window
//! fetched is handed to the document's validation and enforced there, a copy one octet away is
//! said and not enforced, and at `refuse` nothing is fetched and the note says so (ADR 2624).
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
    signature_policy_handed,
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

/// The sentence an outcome ends with where the server answered with a copy, which the window hands
/// to the document's validation as `viewer_core::Command::PolicyCopy` (ADR 2624).
const HANDED: &str = "§12.8.3.4.4 also requires a signature handler to enforce the policy's \
                      constraints: the copy goes on to this document's validation, which enforces \
                      them from it where it is bound to the signature and says what came of them";

/// The sentence an outcome ends with where no copy came back, or none was fetched.
const NOT_HANDED: &str = "§12.8.3.4.4 also requires a signature handler to enforce the policy's \
                          constraints, and none was enforced from that URL, because no copy came \
                          from it";

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
    assert!(note.contains(HANDED), "{note}");
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
    assert!(note.contains(HANDED), "{note}");
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
    assert!(declined.contains(NOT_HANDED), "{declined}");
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
    assert!(note.contains(NOT_HANDED), "{note}");
}

/// A signature whose stated specification is named where its copy does not match: the one shape
/// no signature on this disk has, built so the other arm is not dead.
#[test]
fn a_named_specification_is_said_where_a_copy_does_not_match() {
    let mut altered = POLICY.to_vec();
    altered[1] ^= 0x01;
    let (base, server) = serve_once("200 OK", "application/pdf", altered);
    let mut policy = published(&format!("{base}/politica_de_firma.pdf"));
    policy.specification = Some(Specification::ObjectIdentifier("1.2.3.6".to_owned()));
    let Reply::Say(note) = fetch_signature_policy(&policy, None) else {
        panic!("a copy that does not hash to the signed digest is not opened");
    };
    let _ = server.join();
    assert!(
        note.contains("which this signature names as the specification identified by 1.2.3.6"),
        "{note}"
    );
}

/// A copy a caller of the C interface fetched itself is compared as a window's is, and its
/// sentence says the constraints were not enforced from it, because no call of that interface
/// hands a copy to the document's validation (ADR 2624).
#[test]
fn a_copy_a_c_caller_was_handed_is_compared_and_not_enforced() {
    let policy = published("https://administracionelectronica.gob.es/politica_de_firma.pdf");
    let Reply::Document { note, .. } = signature_policy_handed(&policy, POLICY.to_vec()) else {
        panic!("a bound PDF is opened beside the document");
    };
    assert!(
        note.contains("that this program was handed (142 byte(s)), and that copy is the document"),
        "{note}"
    );
    assert!(
        note.contains(
            "none was enforced from this copy: this program's C interface has no call that hands \
             a copy to the document's validation"
        ),
        "{note}"
    );
}

/// How long a loopback answer may take before the test calls it lost. A bound on a failure only:
/// the wait ends at the submitter's wake, which comes as soon as the answer is on its channel.
const ANSWERED_WITHIN: std::time::Duration = std::time::Duration::from_mins(1);

/// `Submitter::fetch_policy` for document 7, with the wake a window's loop is given sending on the
/// channel returned, so the test looks once the answer is there rather than on a timer of its own
/// (ADR 1800).
fn fetch_policy_woken(
    submitter: &mut viewer_host::submit::Submitter,
    policy: PublishedPolicy,
) -> std::sync::mpsc::Receiver<()> {
    let (wake, woken) = std::sync::mpsc::channel();
    submitter
        .fetch_policy(
            viewer_core::DocumentId(7),
            policy,
            None,
            Some(Box::new(move || {
                // A test that already failed has dropped the receiver, and nobody is owed a wake.
                let _ = wake.send(());
            })),
        )
        .expect("a thread to fetch on");
    woken
}

/// The one answer the wake on `woken` says has arrived.
fn collected(
    submitter: &mut viewer_host::submit::Submitter,
    woken: &std::sync::mpsc::Receiver<()>,
) -> viewer_host::submit::Returned {
    woken
        .recv_timeout(ANSWERED_WITHIN)
        .expect("the loopback answers and the fetch wakes the window");
    let [returned] = <[_; 1]>::try_from(submitter.collect())
        .expect("the wake comes after the answer is on the channel, and there is one");
    returned
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
    let woken = fetch_policy_woken(&mut submitter, policy);
    assert_eq!(
        submitter.interval(),
        Some(viewer_host::submit::LOOK),
        "an answer is outstanding, so the window's timer runs"
    );
    let returned = collected(&mut submitter, &woken);
    let _ = server.join();
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
    assert!(note.contains(HANDED), "{note}");
}

/// The control (trap 13): the same window path to a server that answers 404 says so and opens
/// nothing, so the test above cannot pass on a reply that ignored the server.
#[test]
fn a_window_s_fetch_of_a_policy_that_is_not_found_is_said() {
    let (base, server) = serve_once("404 Not Found", "text/html", b"<p>no</p>".to_vec());
    let policy = published(&format!("{base}/politica_de_firma.pdf"));
    let mut submitter = viewer_host::submit::Submitter::new();
    let woken = fetch_policy_woken(&mut submitter, policy);
    let returned = collected(&mut submitter, &woken);
    let _ = server.join();
    let Reply::Say(note) = returned.reply() else {
        panic!("a 404 is said, not opened");
    };
    assert!(note.contains("answered 404"), "{note}");
}

/// The policy document the finish line's loopback serves: ETSI TS 119 172-2's XML syntax, under
/// the identifier the fixture's signature names, with a signing period that ended in 2021 and a
/// rule that the signature carry its signing certificate (clause 5.2.4's `signingCertOnly`).
const XML_POLICY: &str = r#"<!--a--><SignaturePolicy xmlns="http://uri.etsi.org/19172/v1.1.1#"
    xmlns:ds="http://www.w3.org/2000/09/xmldsig#"><Digest>
    <ds:DigestMethod Algorithm="http://www.w3.org/2001/04/xmlenc#sha256"/>
    <ds:DigestValue>AAAA</ds:DigestValue>
    <ds:CanonicalizationMethod Algorithm="http://www.w3.org/2001/10/xml-exc-c14n#"/>
    </Digest><PolicyComponents><GeneralDetails><SigPolicyDetails>
    <PolicyIdentifier>urn:oid:2.16.724.1.3.1.1.2.1.9</PolicyIdentifier></SigPolicyDetails>
    <OtherDetails><DateOfIssue>2020-01-01T00:00:00Z</DateOfIssue><SigningPeriod>
    <NotBefore>2020-01-01T00:00:00Z</NotBefore><NotAfter>2021-01-01T00:00:00Z</NotAfter>
    </SigningPeriod></OtherDetails></GeneralDetails><PolicyRules><PolicyRule>
    <SigningCertRules><MandatedSigningCertInfo>signingCertOnly</MandatedSigningCertInfo>
    <SigningCertTrustConditions><SignerTrustTrees/><SignerRevTrust><CertificateRevReq><EndRevReq>nocheck</EndRevReq><CACerts>nocheck</CACerts></CertificateRevReq></SignerRevTrust></SigningCertTrustConditions></SigningCertRules></PolicyRule></PolicyRules></PolicyComponents>
    </SignaturePolicy>"#;

/// One DER value: a tag, X.690's shortest definite length, and the contents.
fn der(tag: u8, contents: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    let length = contents.len();
    if length < 0x80 {
        out.push(u8::try_from(length).expect("under 128"));
    } else if length < 0x100 {
        out.extend([0x81, u8::try_from(length).expect("under 256")]);
    } else {
        let length = u16::try_from(length).expect("the fixture is short");
        out.push(0x82);
        out.extend(length.to_be_bytes());
    }
    out.extend_from_slice(contents);
    out
}

/// A CMS `SignedData` whose one `SignerInfo` signs §12.8.3.4.4's signature-policy-identifier
/// (ETSI EN 319 122-1 clause 5.2.9.1) under [`IDENTIFIER`] with the SHA-1 of [`XML_POLICY`] and
/// clause 5.2.9.2's URL qualifier naming `url`. Nothing in it verifies, which is why the word valid
/// is not what this file can show (`viewer_core::notes`' anchor test says why); the policy is read
/// and enforced whether or not the signature verifies.
fn policy_cms(url: &str) -> Vec<u8> {
    const SIGNED_DATA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x07, 0x02];
    const DATA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x07, 0x01];
    const SHA256: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01];
    const SHA1: &[u8] = &[0x2b, 0x0e, 0x03, 0x02, 0x1a];
    const RSA: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01];
    const SIG_POLICY_ID: &[u8] = &[
        0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x09, 0x10, 0x02, 0x0f,
    ];
    const SPQ_URI: &[u8] = &[
        0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x09, 0x10, 0x05, 0x01,
    ];
    const POLICY_OID: &[u8] = &[0x60, 0x85, 0x54, 0x01, 0x03, 0x01, 0x01, 0x02, 0x01, 0x09];
    let algorithm = |oid: &[u8]| der(0x30, &der(0x06, oid));
    let digest = Digest::Sha1.compute(&[XML_POLICY.as_bytes()]);
    let qualifier = der(
        0x30,
        &[der(0x06, SPQ_URI), der(0x16, url.as_bytes())].concat(),
    );
    let policy = [
        der(0x06, POLICY_OID),
        der(0x30, &[algorithm(SHA1), der(0x04, &digest)].concat()),
        der(0x30, &qualifier),
    ]
    .concat();
    let attribute = der(
        0x30,
        &[der(0x06, SIG_POLICY_ID), der(0x31, &der(0x30, &policy))].concat(),
    );
    let signer_info = der(
        0x30,
        &[
            der(0x02, &[1]),
            der(0x30, &[der(0x30, &[]), der(0x02, &[1])].concat()),
            algorithm(SHA256),
            der(0xa0, &attribute),
            der(0x30, &[der(0x06, RSA), der(0x05, &[])].concat()),
            der(0x04, &[0; 8]),
        ]
        .concat(),
    );
    let signed_data = der(
        0x30,
        &[
            der(0x02, &[1]),
            der(0x31, &algorithm(SHA256)),
            algorithm(DATA),
            der(0x31, &signer_info),
        ]
        .concat(),
    );
    der(
        0x30,
        &[der(0x06, SIGNED_DATA), der(0xa0, &signed_data)].concat(),
    )
}

/// A one-page document signed under [`policy_cms`], its byte range ending at the revision's end
/// so §12.8.2.2.2's comparison opens it. Object 1 is the catalog, 2 the page tree, 3 a page, 4 the
/// signature and 5 its field.
fn signed_under_a_policy(url: &str) -> Vec<u8> {
    use std::fmt::Write as _;
    let mut contents = String::new();
    for octet in policy_cms(url) {
        let _ = write!(contents, "{octet:02x}");
    }
    let signature = format!(
        "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /ETSI.CAdES.detached \
         /ByteRange [0000000000 0000000000 0000000000 0000000000] /Contents <{contents}> >>"
    );
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] /SigFlags 3 >> >>",
        "<< /Type /Pages /Count 1 /Kids [3 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>",
        signature.as_str(),
        "<< /FT /Sig /T (Signature1) /V 4 0 R /Subtype /Widget >>",
    ];
    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for (index, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", index.saturating_add(1));
    }
    let xref_at = out.len();
    let size = objects.len().saturating_add(1);
    let _ = write!(out, "xref\n0 {size}\n0000000000 65535 f \n");
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n"
    );
    let value = out.find("/Contents <").expect("a signature value");
    let start = value.saturating_add("/Contents ".len());
    let end = out[start..]
        .find('>')
        .map(|at| start.saturating_add(at).saturating_add(1))
        .expect("a closing delimiter");
    let range = out.find("/ByteRange [").expect("a placeholder range");
    let at = range.saturating_add("/ByteRange [".len());
    let filled = format!(
        "{:010} {:010} {:010} {:010}",
        0,
        start,
        end,
        out.len().saturating_sub(end)
    );
    out.replace_range(at..at.saturating_add(filled.len()), &filled);
    out.into_bytes()
}

/// The document opened in a viewer, reported, and the one policy its signature publishes.
fn opened_and_published(url: &str) -> (viewer_core::Viewer, Vec<String>, PublishedPolicy) {
    use viewer_core::{Command, DocumentId, Event, Viewer};
    let mut viewer = Viewer::new(200, 200, 1.0);
    let _ = viewer
        .handle(Command::Open {
            id: DocumentId(7),
            bytes: signed_under_a_policy(url).into(),
            password: None,
            fragment: None,
        })
        .count();
    let mut notes = Vec::new();
    let mut published = Vec::new();
    for event in viewer.handle(Command::Report) {
        match event {
            Event::Reported {
                page: None,
                notes: said,
                ..
            } => notes.extend(said),
            Event::SignaturePoliciesPublished { policies, .. } => published.extend(policies),
            _ => {}
        }
    }
    let [policy] = <[_; 1]>::try_from(published).expect("the signature publishes one policy");
    (viewer, notes, policy)
}

/// What the document's validation says once the window hands it what the server answered.
fn enforced_from(
    viewer: &mut viewer_core::Viewer,
    returned: &viewer_host::submit::Returned,
) -> Vec<String> {
    let command = returned
        .follow_up()
        .expect("a 2xx answer to a policy fetch is handed on");
    viewer
        .handle(command)
        .flat_map(|event| match event {
            viewer_core::Event::Reported { notes, .. } => notes,
            _ => Vec::new(),
        })
        .collect()
}

/// **The finish line**: at `send` the window fetches the copy the signature names from the
/// loopback, hands it to the document's validation, and the policy's constraints are enforced from
/// it. The signature carries no certificate, so the `signingCertOnly` rule is broken and said to
/// be; the signing period that ended in 2021 is said and not evaluated, because nothing in this
/// fixture establishes when it was signed (only a signature timestamp covering it does, and none
/// is built here).
///
/// The word valid being withheld under an ended signing period is proven in `pdf_signature`'s
/// `a_fetched_policy_copy_is_enforced_as_a_stored_one_is_and_a_changed_one_is_not`, over the
/// verdict fixture: no signed document a viewer opens carries a signature that verifies to an
/// anchor, so no end-to-end verdict exists here to withhold it from.
///
/// **The plant** (trap 13) is the same exchange with one octet of the served copy changed, inside
/// the XML comment so every rule in it still reads: it is said not to hash to the signed digest
/// and nothing in it is enforced.
#[test]
fn a_policy_the_window_fetched_is_enforced_by_the_document_s_validation() {
    let enforce = |served: Vec<u8>| {
        let (base, server) = serve_once("200 OK", "application/xml", served);
        let url = format!("{base}/policy.xml");
        let (mut viewer, before, policy) = opened_and_published(&url);
        assert_eq!(
            may_fetch_signature_policy(&policy, Submissions::Send),
            Sending::Send
        );
        let mut submitter = viewer_host::submit::Submitter::new();
        let woken = fetch_policy_woken(&mut submitter, policy);
        let returned = collected(&mut submitter, &woken);
        let _ = server.join();
        let after = enforced_from(&mut viewer, &returned);
        let Reply::Say(note) = returned.reply() else {
            panic!("a copy that is not a PDF is said, not opened");
        };
        assert!(note.contains(HANDED), "{note}");
        (url, before, after)
    };

    let (url, before, after) = enforce(XML_POLICY.as_bytes().to_vec());
    assert!(
        !before.iter().any(|note| note.contains("states them in")),
        "nothing is enforced before the copy arrives: {before:#?}"
    );
    let said = after.join("\n");
    assert!(
        said.contains(&format!(
            "the copy fetched from {url} states them in ETSI TS 119 172-2's XML syntax"
        )),
        "{said}"
    );
    // Clause 4.23.2's revocation requirement is one check per kind of certificate, each its own
    // constraint (ADR 2720): nocheck for both is two constraints met.
    assert!(said.contains("5 constraint(s), 1 of them broken"), "{said}");
    // The signing-certificate trust condition is searched on a path to the policy's own trust
    // points (ADR 2714), and with no host clock and no signature timestamp it says so by name.
    assert!(
        said.contains(
            "the policy's signing certificate trust condition is not evaluated: no instant was \
             given to validate the path at"
        ),
        "{said}"
    );
    assert!(
        said.contains(
            "the policy's requirement that the signature carry the signer's certificate is broken"
        ),
        "{said}"
    );
    assert!(
        said.contains(
            "the policy's signing period, from 2020-01-01T00:00:00Z to 2021-01-01T00:00:00Z, is \
             not evaluated: nothing establishes when the signature was made"
        ),
        "{said}"
    );

    let mut changed = XML_POLICY.as_bytes().to_vec();
    changed[5] = b'b';
    let (url, _, after) = enforce(changed);
    let said = after.join("\n");
    assert!(
        said.contains(&format!(
            "this program did not enforce the copy fetched from {url}: the copy fetched from \
             {url} does not hash to the SHA1 digest the signer signed over it"
        )),
        "{said}"
    );
    assert!(!said.contains("states them in"), "{said}");
}

/// The same document at `refuse`: the level answers before any request is made, so the listener
/// never sees a connection, and the note says no copy came from the URL.
#[test]
fn at_refuse_the_policy_is_not_fetched_and_the_note_says_none_was_enforced() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("the loopback takes a listener");
    let url = format!(
        "http://127.0.0.1:{}/policy.xml",
        listener.local_addr().expect("a bound port").port()
    );
    let (_, _, policy) = opened_and_published(&url);
    let Sending::Refuse(why) = may_fetch_signature_policy(&policy, Submissions::Refuse) else {
        panic!("refuse fetches nothing");
    };
    let declined = signature_policy_declined(&policy, &why);
    assert!(declined.contains(NOT_HANDED), "{declined}");
    listener
        .set_nonblocking(true)
        .expect("the listener can be polled");
    assert_eq!(
        listener.accept().map(|_| ()).map_err(|error| error.kind()),
        Err(std::io::ErrorKind::WouldBlock),
        "nothing connected"
    );
}
