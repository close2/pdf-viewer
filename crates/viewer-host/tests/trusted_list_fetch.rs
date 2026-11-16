//! The European Union's trusted lists fetched by this host into the directory a person named, at
//! the reader's network level (`viewer_host::trusted_list_fetch`, ADR 2882).
//!
//! What is fetched and what is kept are `pdf_signature::trusted_list::refresh`'s, pinned there
//! against lists signed with that crate's test keys; what these hold is the host's half: the
//! level's four answers and the refusal before it, a transport that is never asked without the
//! Official Journal's certificates, an answer that is not a list or not a 2xx leaving the directory
//! as it was — over a listener on this machine's loopback, through the submit client — and the
//! persisting of a refresh's answer.
//!
//! The server is a `std::net::TcpListener` bound to `127.0.0.1:0` and read by hand, as
//! `signature_policy.rs`'s is.
//!
//! **A window's refresh** ([`Refresher`], ADR 2886) is held to what it hands the core: the lists in
//! `trusted_lists/` were written by `pdf-signature`'s own fixture generator with the test keys that
//! crate generates — a list of trusted lists whose certificate is the Official Journal's set, and
//! `AT`'s and `BE`'s lists, each signed by the certificate the LOTL's pointer names, and a second
//! `BE` signed by another — and served by a transport at the locations the LOTL names.

#![expect(
    clippy::expect_used,
    reason = "a test's failure is its purpose, and these helpers run outside #[test] bodies \
              where `allow-panic-in-tests` does not reach"
)]

use std::io::{BufRead as _, BufReader, Write as _};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use pdf_signature::trusted_list::Outcome;
use pdf_signature::trusted_list::authentication::OJEU_LOTL_LOC_AT_PUBLICATION;
use pdf_signature::trusted_list::authentication::{OJEU_LOTL_LOC, stored_name};
use pdf_signature::trusted_list::refresh::Refresh;
use pdf_signature::x509::Instant;
use viewer_host::trusted_list_fetch::{
    Refreshed, Refresher, Transport, fetch_trusted_lists, may_fetch_trusted_lists, persist,
    refresh_trusted_lists,
};
use viewer_host::{Sending, Submissions, Trusting};

/// 2026-11-16T00:00:00Z.
const NOW: i64 = 1_794_787_200;

/// A directory of its own under the system's temporary directory, emptied first.
fn directory(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "viewer-host-trusted-list-fetch-{}-{name}",
        std::process::id()
    ));
    // A directory left by an earlier run is the one thing to clear; its absence is the usual case.
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("a temporary directory");
    directory
}

/// The directory's files, by name, sorted.
fn files(directory: &Path) -> Vec<(String, Vec<u8>)> {
    let mut found: Vec<(String, Vec<u8>)> = std::fs::read_dir(directory)
        .expect("the directory lists")
        .flatten()
        .map(|entry| {
            (
                entry.file_name().to_string_lossy().into_owned(),
                std::fs::read(entry.path()).expect("the file reads"),
            )
        })
        .collect();
    found.sort();
    found
}

/// A listener that answers one request with `status` and `body`, and hands back the request line.
fn serve_once(status: &'static str, body: Vec<u8>) -> (String, std::thread::JoinHandle<String>) {
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
            "HTTP/1.1 {status}\r\nContent-Type: application/xml\r\nContent-Length: {}\r\n\
             Connection: close\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(&body);
        line.trim_end().to_owned()
    });
    (format!("http://127.0.0.1:{port}"), server)
}

/// A directory holding an Official Journal certificate file — X.690's `SEQUENCE` tag is what makes
/// it read as DER — and `OJEU-LOTL-Loc` naming `lotl`.
fn configured(name: &str, lotl: &str) -> PathBuf {
    let directory = directory(name);
    std::fs::write(directory.join("OJEU-LOTL-Certs-Set.der"), [0x30, 0x00])
        .expect("the certificate file");
    std::fs::write(directory.join(OJEU_LOTL_LOC), lotl).expect("the location file");
    directory
}

/// The level's four answers, as `doc/questions/A98` set them for every fetch under it, and the
/// refusal at every level where no directory was named to keep the lists in.
#[test]
fn the_level_decides_and_no_directory_refuses_at_every_level() {
    let directory = directory("level");
    assert!(matches!(
        may_fetch_trusted_lists(Some(&directory), Submissions::Refuse),
        Sending::Refuse(_)
    ));
    let Sending::Ask(question) = may_fetch_trusted_lists(Some(&directory), Submissions::Ask) else {
        panic!("ask puts the question");
    };
    assert!(question.reasons.contains(&directory.display().to_string()));
    assert!(matches!(
        may_fetch_trusted_lists(Some(&directory), Submissions::Warn),
        Sending::Warn(_)
    ));
    assert_eq!(
        may_fetch_trusted_lists(Some(&directory), Submissions::Send),
        Sending::Send
    );
    for level in [
        Submissions::Refuse,
        Submissions::Ask,
        Submissions::Warn,
        Submissions::Send,
    ] {
        let Sending::Refuse(why) = may_fetch_trusted_lists(None, level) else {
            panic!("no directory refuses at {level:?}");
        };
        assert!(why.contains("--trusted-lists"), "{why}");
    }
}

/// GPR-4.0-02's `OJEU-LOTL-Certs-Set` has no default: without it nothing is asked of the
/// transport, and the directory is as it was.
#[test]
fn without_the_official_journals_certificates_the_transport_is_never_asked() {
    let directory = directory("no-certificates");
    std::fs::write(directory.join("TL-AT.xml"), b"held").expect("a held list");
    let before = files(&directory);
    let mut asked = 0_usize;
    let mut obtain = |_: &str| {
        asked = asked.saturating_add(1);
        Err("not reached".to_owned())
    };
    let said = refresh_trusted_lists(&directory, Instant::from_unix_seconds(NOW), &mut obtain);
    assert_eq!(asked, 0);
    assert!(
        said.iter().any(
            |line| line.contains("nothing was fetched") && line.contains("OJEU-LOTL-Certs-Set")
        ),
        "{said:?}"
    );
    assert_eq!(files(&directory), before);
}

/// PRO-4.1.4-01 through the submit client: the GET goes to `OJEU-LOTL-Loc`, a 404 is said with
/// its status, and nothing in the directory changes.
#[test]
fn a_lotl_the_server_does_not_answer_with_keeps_the_directory_as_it_was() {
    let (base, server) = serve_once("404 Not Found", b"gone".to_vec());
    let lotl = format!("{base}/eu-lotl.xml");
    let directory = configured("not-found", &lotl);
    let before = files(&directory);
    let said = fetch_trusted_lists(&directory, Instant::from_unix_seconds(NOW), None);
    assert_eq!(
        server.join().expect("the server ran"),
        "GET /eu-lotl.xml HTTP/1.1"
    );
    assert!(
        said.iter()
            .any(|line| line.contains("nothing was kept") && line.contains("answered 404")),
        "{said:?}"
    );
    assert_eq!(files(&directory), before);
}

/// PRO-4.1.4-08's stop: a 2xx answer that is not a signed list fails clause 4.1, and what failed
/// it is not kept — the directory is as it was, and the warning the level gave leads the report.
#[test]
fn an_answer_clause_4_1_does_not_authenticate_is_not_kept() {
    let (base, server) = serve_once("200 OK", b"<not-a-list/>".to_vec());
    let lotl = format!("{base}/eu-lotl.xml");
    let directory = configured("unauthenticated", &lotl);
    let before = files(&directory);
    let said = fetch_trusted_lists(
        &directory,
        Instant::from_unix_seconds(NOW),
        Some("fetched without asking"),
    );
    server.join().expect("the server ran");
    assert_eq!(
        said.first().map(String::as_str),
        Some("trusted lists: fetched without asking")
    );
    assert!(
        said.iter()
            .any(|line| line.contains("LOTL_VERIFICATION_FAILED")
                && line.contains("nothing fetched was kept")),
        "{said:?}"
    );
    assert_eq!(files(&directory), before);
    assert!(
        !files(&directory)
            .iter()
            .any(|(name, _)| name == &stored_name(&lotl))
    );
}

/// A refresh's answer persisted: each file under its name, replaced whole with no part file left,
/// each removal done and said, and a name that is not one component neither written nor removed.
#[test]
fn a_refresh_is_persisted_file_by_file() {
    let directory = directory("persist");
    let lotl = "https://ec.europa.eu/tools/lotl/eu-lotl.xml";
    std::fs::write(directory.join(stored_name(lotl)), b"old").expect("an old copy");
    std::fs::write(directory.join("gone"), b"old").expect("a file to remove");
    std::fs::write(directory.join("kept-by-hand.xml"), b"mine").expect("a person's file");
    let refreshed = Refresh {
        lotl: Outcome {
            status: "LOTL_VERIFICATION_PASSED",
            sub_status: Vec::new(),
        },
        locations: Vec::new(),
        store: vec![
            (stored_name(lotl), b"new".to_vec()),
            (OJEU_LOTL_LOC.to_owned(), lotl.as_bytes().to_vec()),
            ("../escape".to_owned(), b"no".to_vec()),
        ],
        remove: vec!["gone".to_owned(), "..".to_owned()],
        configuration: Vec::new(),
    };
    let said = persist(&directory, &refreshed);
    assert_eq!(
        files(&directory),
        [
            (OJEU_LOTL_LOC.to_owned(), lotl.as_bytes().to_vec()),
            (stored_name(lotl), b"new".to_vec()),
            ("kept-by-hand.xml".to_owned(), b"mine".to_vec()),
        ]
    );
    assert!(
        said.iter().any(|line| line.contains("gone was removed")),
        "{said:?}"
    );
    assert!(
        said.iter()
            .any(|line| line.contains("../escape is not a file name")),
        "{said:?}"
    );
    assert!(
        !directory
            .parent()
            .expect("a parent")
            .join("escape")
            .exists()
    );
}

/// The Member State lists the LOTL in `trusted_lists/` points at.
const AT_TSL: &str = "https://example.at/tsl.xml";
const BE_TSL: &str = "https://example.be/tsl.xml";

/// One of `trusted_lists/`'s files.
fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/trusted_lists")
            .join(name),
    )
    .expect("the fixture reads")
}

/// A directory holding the Official Journal's certificate alone, as a reader who has fetched
/// nothing yet holds it, and the words naming it.
fn trusting(name: &str) -> (PathBuf, Trusting) {
    let directory = directory(name);
    std::fs::write(
        directory.join("OJEU-LOTL-Certs-Set.cer"),
        fixture("OJEU-LOTL-Certs-Set.cer"),
    )
    .expect("the certificate file");
    let trusting = Trusting {
        trusted_lists: Some(directory.clone()),
        ..Trusting::default()
    };
    (directory, trusting)
}

/// A transport serving the LOTL, `AT`'s list, and `be` as `BE`'s.
fn serving(be: &'static str) -> Transport {
    Box::new(move |uri: &str| match uri {
        OJEU_LOTL_LOC_AT_PUBLICATION => Ok(fixture("eu-lotl.xml")),
        AT_TSL => Ok(fixture("at-tsl.xml")),
        BE_TSL => Ok(fixture(be)),
        other => Err(format!("{other} answered 404")),
    })
}

/// The refresh's answer, waited for as a window's timer looks for it.
fn collected(out: &mut Refresher) -> Refreshed {
    for _ in 0..400 {
        if let Some(refreshed) = out.collect() {
            assert!(out.interval().is_none(), "nothing is out once it is in");
            return refreshed;
        }
        assert!(out.interval().is_some(), "the refresh is out");
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    out.collect()
        .expect("the refresh came back inside ten seconds")
}

/// The names of the lists a policy carries.
fn names(trust: &viewer_core::TrustPolicy) -> Vec<String> {
    trust
        .lists
        .lists()
        .iter()
        .map(|(name, _)| name.clone())
        .collect()
}

/// TS 119 615 PRO-4.1.4-01 and PRO-4.2.4-03 through a window's refresh: what clauses 4.1 and 4.2
/// authenticate is kept, and the policy that crosses back to the core as `Command::Trust` is read
/// from the directory afterwards, so it carries the LOTL and both Member States' lists the reader
/// did not hold before.
#[test]
fn a_refresh_hands_the_core_the_lists_it_kept() {
    let (directory, trusting) = trusting("window-kept");
    let (before, _) = trusting.policy();
    assert_eq!(names(&before), ["OJEU-LOTL-Certs-Set.cer"]);
    let mut out = Refresher::new();
    out.start_with(
        trusting,
        Instant::from_unix_seconds(NOW),
        None,
        serving("be-tsl.xml"),
        None,
    )
    .expect("a refresh starts");
    let refreshed = collected(&mut out);
    assert!(
        refreshed
            .said
            .iter()
            .any(|line| line.contains("LOTL_VERIFICATION_PASSED")),
        "{:?}",
        refreshed.said
    );
    let after = names(&refreshed.trust);
    for uri in [OJEU_LOTL_LOC_AT_PUBLICATION, AT_TSL, BE_TSL] {
        assert!(after.contains(&stored_name(uri)), "{uri} in {after:?}");
    }
    assert_eq!(
        refreshed
            .trust
            .lists
            .lists()
            .iter()
            .find(|(name, _)| name == &stored_name(AT_TSL))
            .map(|(_, octets)| octets.clone()),
        Some(fixture("at-tsl.xml"))
    );
    assert_eq!(
        refreshed.trust.lists.source(),
        format!("{} (--trusted-lists)", directory.display())
    );
}

/// PRO-4.2.4-07: a Member State's list its pointer's certificate did not sign is not kept, so the
/// core is not handed it — `BE`'s alone differs from the test above, and `AT`'s still crosses.
#[test]
fn a_list_clause_4_2_refuses_is_not_handed_to_the_core() {
    let (_, trusting) = trusting("window-refused");
    let mut out = Refresher::new();
    out.start_with(
        trusting,
        Instant::from_unix_seconds(NOW),
        None,
        serving("be-tsl-other-signer.xml"),
        None,
    )
    .expect("a refresh starts");
    let refreshed = collected(&mut out);
    let after = names(&refreshed.trust);
    assert!(after.contains(&stored_name(AT_TSL)), "{after:?}");
    assert!(!after.contains(&stored_name(BE_TSL)), "{after:?}");
}

/// One refresh at a time — two would write one directory — and none without a directory to write.
#[test]
fn a_second_refresh_and_one_with_no_directory_are_declined() {
    let (_, trusting) = trusting("window-second");
    let mut out = Refresher::new();
    let (release, held) = std::sync::mpsc::channel::<()>();
    let held = std::sync::Mutex::new(held);
    let transport: Transport = Box::new(move |_: &str| {
        // Held until the test has pressed twice; a dropped sender also lets it go.
        let _ = held.lock().map(|held| held.recv());
        Err("held".to_owned())
    });
    out.start_with(
        trusting.clone(),
        Instant::from_unix_seconds(NOW),
        None,
        transport,
        None,
    )
    .expect("the first starts");
    let second = out
        .start_with(
            trusting,
            Instant::from_unix_seconds(NOW),
            None,
            serving("be-tsl.xml"),
            None,
        )
        .expect_err("the second is declined");
    assert!(second.contains("already under way"), "{second}");
    release.send(()).expect("the transport is waiting");
    collected(&mut out);
    let none = Refresher::new()
        .start(
            Trusting::default(),
            Instant::from_unix_seconds(NOW),
            None,
            None,
        )
        .expect_err("nowhere to keep them");
    assert!(none.contains("--trusted-lists"), "{none}");
}
