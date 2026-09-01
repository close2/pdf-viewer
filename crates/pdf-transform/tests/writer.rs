//! `attachments --attach`: §7.5.6's incremental update carrying one new embedded file, held to
//! the clauses it writes against — the source's bytes intact under the update, the file read
//! back by this tree's own reader with Table 45's size and checksum agreeing with the bytes,
//! §7.9.6's order in the name tree, and determinism (RFC 0002 section 9). `qpdf --check`, where
//! it is installed, is evidence about the reading and never its definition (principle 5).

#![expect(
    clippy::expect_used,
    clippy::print_stderr,
    reason = "test code: a fixture that cannot exercise the rule must fail loudly, and a \
              skipped test says so"
)]

mod support;

use std::process::Command;

use pdf_model::attachment::attachments;
use pdf_syntax::Document;
use pdf_transform::attachments::{Action, AttachmentsPlan, Payload, parse_iso_8601};
use pdf_transform::{Budget, Exit, MemorySinks, Origin, Plan, Policy, Refusal, Source, apply};

use support::committed;

/// Attaches `payload` under `name` to `bytes`, answering the whole updated file.
fn attach(
    bytes: &[u8],
    payload: &[u8],
    name: &str,
    description: Option<&str>,
    date: Option<&str>,
) -> Result<Vec<u8>, Refusal> {
    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Attachments(AttachmentsPlan {
            source: 0,
            action: Action::Attach {
                payload: Payload::new(payload.to_vec()),
                name: name.to_owned(),
                description: description.map(str::to_owned),
                date: date.map(|text| parse_iso_8601(text).expect("a well-formed date")),
                names: "out.pdf".parse().expect("a pattern"),
            },
        }),
        &[Source::new(bytes.to_vec())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )?;
    assert_eq!(report.exit(false, false), Exit::Success, "{report:?}");
    assert!(matches!(
        &report.outputs[..],
        [pdf_transform::Output {
            origin: Origin::Updated { source: 0, attached },
            ..
        }] if attached == name
    ));
    let mut outputs = sinks.into_outputs();
    assert_eq!(outputs.len(), 1);
    Ok(outputs.remove(0).1)
}

/// `qpdf --check` on the file, where qpdf is installed: `Some(accepted)`.
fn qpdf_accepts(path: &std::path::Path) -> Option<bool> {
    let output = Command::new("qpdf")
        .arg("--check")
        .arg(path)
        .output()
        .ok()?;
    Some(output.status.success())
}

/// The names §7.7.4's tree files the document's embedded files under, in tree order.
fn tree_names(document: &Document) -> Vec<String> {
    attachments(document)
        .into_iter()
        .map(|attachment| attachment.name)
        .collect()
}

/// A committed document that states no `/Names` at all: the catalog is the holder that is
/// rewritten. The source's bytes stay byte for byte under the update, the reader reads the
/// file back with Table 45's `/Size` and `/CheckSum` agreeing with the bytes, and no date is
/// written unless one was given.
#[test]
fn a_file_is_attached_by_an_update_the_source_stays_under_and_this_reader_reads_back() {
    let source = std::fs::read(committed("PDF20_AN001-BPC.pdf")).expect("a committed document");
    let payload = b"hello, attachment\n";
    let updated = attach(&source, payload, "notes.txt", Some("a note"), None).expect("attached");

    // §7.5.6: "leaving its original contents intact".
    assert!(updated.len() > source.len());
    assert_eq!(&updated[..source.len()], &source[..]);
    assert!(
        !updated[source.len()..].windows(8).any(|w| w == b"/ModDate"),
        "no date was given, so none is written"
    );

    let document = Document::open(updated.clone()).expect("the update opens");
    let files = attachments(&document);
    let [file] = &files[..] else {
        panic!("one embedded file, and the reader found {files:?}");
    };
    assert_eq!(file.name, "notes.txt");
    assert_eq!(file.file_name.as_deref(), Some("notes.txt"));
    assert_eq!(file.description.as_deref(), Some("a note"));
    assert_eq!(
        file.size,
        Some(i64::try_from(payload.len()).expect("small"))
    );
    assert_eq!(file.created, None);
    let read_back = document
        .decoded_stream_data(&file.stream)
        .expect("the stream decodes");
    assert_eq!(&read_back[..], &payload[..]);
    assert_eq!(
        file.checksum_matches(&read_back),
        Some(true),
        "Table 45's /CheckSum is MD5 of the bytes"
    );

    // RFC 0002 section 9: the same plan is the same bytes.
    let again = attach(&source, payload, "notes.txt", Some("a note"), None).expect("attached");
    assert_eq!(updated, again);

    let dir = std::env::temp_dir().join(format!("pdf-transform-writer-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("writable");
    let path = dir.join("attached.pdf");
    std::fs::write(&path, &updated).expect("written");
    match qpdf_accepts(&path) {
        Some(accepted) => assert!(accepted, "qpdf --check does not accept the update"),
        None => eprintln!("qpdf is not installed; the foreign-reader evidence was not taken"),
    }
}

/// `--date`: Table 45's `/CreationDate` and `/ModDate` in §7.9.4's form, read back as the
/// instant that was typed.
#[test]
fn a_date_given_is_written_in_the_clauses_form_and_read_back() {
    let source = std::fs::read(committed("PDF20_AN001-BPC.pdf")).expect("a committed document");
    let updated = attach(
        &source,
        b"x",
        "dated.bin",
        None,
        Some("2026-09-01T12:34:56+02:00"),
    )
    .expect("attached");
    let document = Document::open(updated).expect("opens");
    let [file] = &attachments(&document)[..] else {
        panic!("one file");
    };
    assert_eq!(file.modified.as_deref(), Some("D:20260901123456+02'00'"));
    assert_eq!(file.created, file.modified);
    let date = file.modified_date().expect("§7.9.4 parses it");
    assert_eq!((date.year, date.month, date.day), (2026, 9, 1));
    assert_eq!((date.hour, date.minute, date.second), (12, 34, 56));
    assert_eq!(date.offset, Some(120));

    assert_eq!(
        parse_iso_8601("2026-09-01T00:00:00Z").map(|d| d.offset),
        Some(Some(0))
    );
    assert_eq!(
        parse_iso_8601("2026-09-01T00:00:00-05:30").map(|d| d.offset),
        Some(Some(-330))
    );
    assert_eq!(
        parse_iso_8601("2026-09-01T00:00:00").map(|d| d.offset),
        Some(None)
    );
    for malformed in [
        "2026-09-01",
        "2026-13-01T00:00:00",
        "yesterday",
        "2026-09-01T25:00:00Z",
    ] {
        assert!(parse_iso_8601(malformed).is_none(), "{malformed:?}");
    }
}

/// A corpus document that already has an `/EmbeddedFiles` tree: two updates in a row, each
/// leaving its input intact, the tree read back in §7.9.6's order — "[t]he keys shall be sorted
/// in lexical order" — whatever order the files were added in; and a name the tree already
/// holds is refused rather than doubled.
#[test]
fn attaching_into_an_existing_tree_keeps_its_files_and_the_clauses_order() {
    let Some(path) = support::corpus("attachment.pdf") else {
        eprintln!("skipped: the pdf.js corpus is not checked out");
        return;
    };
    let source = std::fs::read(path).expect("a corpus document");
    let before = tree_names(&Document::open(source.clone()).expect("opens"));
    assert_eq!(before, ["foo.txt"], "the fixture files one");

    let once = attach(&source, b"last", "zzz.txt", None, None).expect("attached");
    assert_eq!(&once[..source.len()], &source[..]);
    let twice = attach(&once, b"first", "aaa.txt", None, None).expect("attached again");
    assert_eq!(&twice[..once.len()], &once[..]);

    let document = Document::open(twice.clone()).expect("opens");
    assert_eq!(tree_names(&document), ["aaa.txt", "foo.txt", "zzz.txt"]);
    let foo = attachments(&document)
        .into_iter()
        .find(|file| file.name == "foo.txt")
        .expect("the fixture's own file is still filed");
    assert_eq!(foo.size, Some(9), "as the fixture stated it");

    match attach(&twice, b"again", "foo.txt", None, None) {
        Err(Refusal::AttachmentExists { name, .. }) => assert_eq!(name, "foo.txt"),
        other => panic!("a name the tree holds is refused: {other:?}"),
    }

    let dir =
        std::env::temp_dir().join(format!("pdf-transform-writer-{}-tree", std::process::id()));
    std::fs::create_dir_all(&dir).expect("writable");
    let path = dir.join("twice.pdf");
    std::fs::write(&path, &twice).expect("written");
    if let Some(accepted) = qpdf_accepts(&path) {
        assert!(accepted, "qpdf --check does not accept the chained updates");
    }
}

/// Through the program: the flags, the exit status, and the listing of the output.
#[test]
fn the_program_attaches_and_lists_it_back() {
    let dir = std::env::temp_dir().join(format!("pdf-transform-writer-{}-cli", std::process::id()));
    std::fs::create_dir_all(&dir).expect("writable");
    std::fs::write(dir.join("report.csv"), "a,b\n1,2\n").expect("written");
    let source = committed("PDF20_AN001-BPC.pdf");
    let output = Command::new(env!("CARGO_BIN_EXE_pdf-transform"))
        .args([
            "attachments",
            source.to_str().expect("utf-8"),
            "--attach",
            "report.csv",
            "--name",
            "q3.csv",
            "--description",
            "third quarter",
            "-o",
            "out.pdf",
        ])
        .current_dir(&dir)
        .output()
        .expect("runs");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let listing = Command::new(env!("CARGO_BIN_EXE_pdf-transform"))
        .args(["attachments", "out.pdf", "--list"])
        .current_dir(&dir)
        .output()
        .expect("runs");
    assert_eq!(listing.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&listing.stdout),
        "q3.csv\tq3.csv\t8\t-\n"
    );

    // Attaching it a second time under the same name is exit 2: the file defeated the request.
    let again = Command::new(env!("CARGO_BIN_EXE_pdf-transform"))
        .args([
            "attachments",
            "out.pdf",
            "--attach",
            "report.csv",
            "--name",
            "q3.csv",
            "-o",
            "out2.pdf",
        ])
        .current_dir(&dir)
        .output()
        .expect("runs");
    assert_eq!(again.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&again.stderr).contains("already named \"q3.csv\""));
}
