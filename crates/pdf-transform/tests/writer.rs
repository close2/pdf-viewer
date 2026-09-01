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
use pdf_syntax::{Document, Object};
use pdf_transform::attachments::{Action, AttachmentsPlan, OnPage, Payload, parse_iso_8601};
use pdf_transform::range::Selection;
use pdf_transform::render::{ImageFormat, RenderPlan, Sizing};
use pdf_transform::{
    Budget, Exit, Level, MemorySinks, Origin, Plan, Policy, Refusal, Source, apply,
};

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
                on_page: None,
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

/// The four levels against the two kinds of restriction, asked once in `apply`.
///
/// `bug1815476.pdf` is encrypted with `/P -1084` — Table 22 bit 4 clear — and
/// `xfa_filled_imm1344e.pdf` carries the corpus's one `/Perms /DocMDP`, at Table 257's level 2,
/// "filling in forms, instantiating page templates, and signing"; writing a file into it is
/// none of those, so the certification withholds `Modify` while §12.8.6 says nothing about
/// reading it back. `Level::Ask` is the level a pipe cannot answer, and the answer it gives
/// instead is its own refusal rather than `On`'s.
#[test]
fn every_level_is_answered_against_encryption_and_certification() {
    let (Some(encrypted), Some(certified)) = (
        support::corpus("bug1815476.pdf"),
        support::corpus("xfa_filled_imm1344e.pdf"),
    ) else {
        eprintln!("skipped: the pdf.js corpus is not checked out");
        return;
    };
    let encrypted = std::fs::read(encrypted).expect("a corpus document");
    let certified = std::fs::read(certified).expect("a corpus document");

    let attach_under = |bytes: &[u8], level: Level| {
        let sinks = MemorySinks::new();
        let report = apply(
            &Plan::Attachments(AttachmentsPlan {
                source: 0,
                action: Action::Attach {
                    payload: Payload::new(b"x".to_vec()),
                    name: "level.txt".to_owned(),
                    description: None,
                    date: None,
                    names: "out.pdf".parse().expect("a pattern"),
                    on_page: None,
                },
            }),
            &[Source::new(bytes.to_vec())],
            &sinks,
            &Policy {
                restrictions: level,
            },
            &Budget::default(),
        );
        (report, sinks.into_outputs().len())
    };

    // Encryption, Table 22 bit 4.
    match attach_under(&encrypted, Level::On) {
        (Err(Refusal::Restricted { operation, reasons }), 0) => {
            assert_eq!(operation, "modifying the document");
            assert_eq!(reasons, "Table 22 bit 4 is clear");
        }
        other => panic!("`on` refuses by name: {other:?}"),
    }
    match attach_under(&encrypted, Level::Ask) {
        (Err(Refusal::Unanswered { reasons, .. }), 0) => {
            assert_eq!(reasons, "Table 22 bit 4 is clear");
        }
        other => panic!("`ask` in a pipe is a question nobody answered: {other:?}"),
    }
    let (report, written) = attach_under(&encrypted, Level::Warn);
    let report = report.expect("`warn` proceeds");
    assert_eq!(written, 1);
    assert_eq!(report.exit(false, false), Exit::Warnings);
    assert_eq!(
        report
            .warnings
            .iter()
            .map(|w| w.detail.as_str())
            .collect::<Vec<_>>(),
        ["this document restricts modifying the document: Table 22 bit 4 is clear"]
    );
    let (report, written) = attach_under(&encrypted, Level::Off);
    assert_eq!(
        report.expect("`off` proceeds").exit(false, false),
        Exit::Success
    );
    assert_eq!(written, 1);

    // Certification, Table 257's /P 2.
    match attach_under(&certified, Level::On) {
        (Err(Refusal::Restricted { reasons, .. }), 0) => assert_eq!(
            reasons,
            "its author's certification permits only form filling and signing (§12.8.2.2's \
             /P 2), not modifying the document"
        ),
        other => panic!("a certification is a restriction §12.8.6 makes binding: {other:?}"),
    }
    let (report, written) = attach_under(&certified, Level::Warn);
    assert_eq!(
        report.expect("`warn` proceeds").exit(false, false),
        Exit::Warnings
    );
    assert_eq!(written, 1);

    // Reading the certified document back is not a change, so no level withholds it.
    let sinks = MemorySinks::new();
    let listing = apply(
        &Plan::Attachments(AttachmentsPlan {
            source: 0,
            action: Action::SaveAll {
                names: "%d".parse().expect("a pattern"),
            },
        }),
        &[Source::new(certified)],
        &sinks,
        &Policy {
            restrictions: Level::On,
        },
        &Budget::default(),
    )
    .expect("extraction is not a change Table 257 names");
    assert!(listing.warnings.is_empty(), "{listing:?}");
}

/// Page 1 of a document rendered at 150 dpi, with or without its annotations.
fn rendered(bytes: &[u8], annotations: bool) -> (u32, u32, Vec<u8>) {
    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Render(RenderPlan {
            source: 0,
            pages: "1".parse::<Selection>().expect("a selection"),
            size: Sizing::Dpi(150.0),
            format: ImageFormat::Png,
            page_box: None,
            annotations,
            names: "p.png".parse().expect("a pattern"),
        }),
        &[Source::new(bytes.to_vec())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("renders");
    assert_eq!(report.exit(false, false), Exit::Success, "{report:?}");
    let mut outputs = sinks.into_outputs();
    support::decode_png(&outputs.remove(0).1)
}

/// `--to-page`: the file filed by a §12.5.6.15 annotation on the page, which this reader lists
/// under that page and draws with its own icon; the name tree gains nothing; the prefix stays.
///
/// The drawing is checked against the same page interpreted without its `/Annots`: the pixels
/// that differ lie inside the rectangle the annotation states and nowhere else, which is what
/// Table 166's `/Rect` — "[t]he annotation rectangle, defining the location of the annotation
/// on the page" — promises about it.
/// Attaches a small CSV to page 1 of `source` by annotation, answering the updated file.
fn attach_to_page(source: &[u8], rect: Option<[f32; 4]>, icon: Option<&str>) -> Vec<u8> {
    let sinks = MemorySinks::new();
    let report = apply(
        &Plan::Attachments(AttachmentsPlan {
            source: 0,
            action: Action::Attach {
                payload: Payload::new(b"a,b\n1,2\n".to_vec()),
                name: "table.csv".to_owned(),
                description: Some("the table's data".to_owned()),
                date: None,
                names: "out.pdf".parse().expect("a pattern"),
                on_page: Some(OnPage {
                    page: 1,
                    rect,
                    icon: icon.map(str::to_owned),
                }),
            },
        }),
        &[Source::new(source.to_vec())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("attached");
    assert_eq!(report.exit(false, false), Exit::Success, "{report:?}");
    sinks.into_outputs().remove(0).1
}

/// A user-space rectangle on a 150 dpi raster whose y runs down from `top`: the device pixels
/// it covers, inclusive, as `(x0, x1, y0, y1)`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a test's rectangle is inside the page, and floor/ceil make the bounds inclusive"
)]
fn device_box(rect: [f32; 4], top: f32) -> (u32, u32, u32, u32) {
    // ISO 32000-2 §8.3.2.3: 72 user-space units to the inch, so 150 dpi is 150/72.
    let scale = 150.0 / 72.0;
    (
        (rect[0] * scale).floor() as u32,
        (rect[2] * scale).ceil() as u32,
        ((top - rect[3]) * scale).floor() as u32,
        ((top - rect[1]) * scale).ceil() as u32,
    )
}

/// How many pixels of `drawn` differ from `bare` inside the device box, and how many outside.
fn differing(
    drawn: &[u8],
    bare: &[u8],
    width: u32,
    height: u32,
    within: (u32, u32, u32, u32),
) -> (usize, usize) {
    let (x0, x1, y0, y1) = within;
    let (mut inside, mut outside) = (0_usize, 0_usize);
    let row = usize::try_from(width).expect("a width").saturating_mul(4);
    for (y, (drawn_row, bare_row)) in drawn
        .chunks_exact(row)
        .zip(bare.chunks_exact(row))
        .enumerate()
    {
        let y = u32::try_from(y).expect("a row");
        for (x, (drawn_px, bare_px)) in drawn_row
            .chunks_exact(4)
            .zip(bare_row.chunks_exact(4))
            .enumerate()
        {
            let x = u32::try_from(x).expect("a column");
            if drawn_px != bare_px {
                if (x0..=x1).contains(&x) && (y0..=y1).contains(&y) {
                    inside = inside.saturating_add(1);
                } else {
                    outside = outside.saturating_add(1);
                }
            }
        }
    }
    assert!(y0 < height, "the box is on the page");
    (inside, outside)
}

#[test]
fn a_file_attached_to_a_page_is_listed_there_and_its_icon_is_drawn_inside_its_rectangle() {
    let source = std::fs::read(committed("PDF20_AN001-BPC.pdf")).expect("a committed document");
    let rect = [100.0_f32, 600.0, 140.0, 640.0];
    let updated = attach_to_page(&source, Some(rect), Some("Graph"));
    assert_eq!(&updated[..source.len()], &source[..]);

    let document = Document::open(updated.clone()).expect("the update opens");
    assert!(
        attachments(&document).is_empty(),
        "filed on the page, not in the tree"
    );
    assert_eq!(
        support::annotation_file_names(&document),
        [(1, "table.csv".to_owned())]
    );
    let sinks = MemorySinks::new();
    let listing = apply(
        &Plan::Attachments(AttachmentsPlan {
            source: 0,
            action: Action::List,
        }),
        &[Source::new(updated.clone())],
        &sinks,
        &Policy::default(),
        &Budget::default(),
    )
    .expect("lists");
    let [pdf_transform::Listed::Attachment(entry)] = &listing.listed[..] else {
        panic!("one file: {listing:?}");
    };
    assert_eq!(entry.page, Some(1));
    assert_eq!(
        entry.description.as_deref(),
        Some("the table's data"),
        "§12.5.6.15: the annotation's /Contents is the description a reader shows"
    );
    // `/Annots` is a reference in this document, so the array's own object was rewritten and
    // the page dictionary says what it said; either way the resolved array ends with the new one.
    let page = &support::page_dictionaries(&document)[0];
    let annots = document.get_key(page, "Annots");
    let annotation = document.resolve(
        annots
            .as_array()
            .and_then(<[Object]>::last)
            .expect("the page's /Annots ends with the new annotation"),
    );
    let annotation = annotation.as_dict().cloned().expect("a dictionary");
    assert_eq!(
        document
            .get_key(&annotation, "Name")
            .as_name()
            .and_then(|n| n.as_str()),
        Some("Graph")
    );
    assert!(
        annotation.get("AP").is_none(),
        "no appearance stream is written; the icon is this tree's own artwork"
    );

    // Trap 1 in miniature: the icon is on the page, and only where the rectangle says.
    let (width, height, drawn) = rendered(&updated, true);
    let (_, _, bare) = rendered(&updated, false);
    assert_ne!(drawn, bare, "the annotation draws something");
    let page = support::page_dictionaries(&document);
    let media = document.get_key(&page[0], "MediaBox");
    let media = media.as_array().expect("a media box");
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a page height is a small integer in every committed document"
    )]
    let top = media[3].as_number().expect("a number") as f32;
    let (inside, outside) = differing(&drawn, &bare, width, height, device_box(rect, top));
    assert!(inside > 0);
    assert_eq!(outside, 0, "nothing outside /Rect changed");

    let dir =
        std::env::temp_dir().join(format!("pdf-transform-writer-{}-page", std::process::id()));
    std::fs::create_dir_all(&dir).expect("writable");
    let path = dir.join("on-page.pdf");
    std::fs::write(&path, &updated).expect("written");
    if let Some(accepted) = qpdf_accepts(&path) {
        assert!(accepted, "qpdf --check does not accept the update");
    }
}

/// Where nobody states a rectangle or an icon: `OnPage`'s documented default square, and Table
/// 187's default name written out, because the table asks writers to include the entry.
#[test]
fn where_nobody_states_a_rectangle_the_default_square_and_icon_are_written() {
    let source = std::fs::read(committed("PDF20_AN001-BPC.pdf")).expect("a committed document");
    let defaulted = Document::open(attach_to_page(&source, None, None)).expect("opens");
    let page = support::page_dictionaries(&defaulted);
    let annots = defaulted.get_key(&page[0], "Annots");
    let last = defaulted.resolve(annots.as_array().expect("an array").last().expect("one"));
    let last = last.as_dict().expect("a dictionary");
    assert_eq!(
        defaulted
            .get_key(last, "Name")
            .as_name()
            .and_then(|n| n.as_str()),
        Some(OnPage::DEFAULT_ICON)
    );
    let stated: Vec<f64> = defaulted
        .get_key(last, "Rect")
        .as_array()
        .expect("a rectangle")
        .iter()
        .map(|v| v.as_number().expect("a number"))
        .collect();
    let side = f64::from(OnPage::DEFAULT_SIDE);
    assert!((stated[2] - stated[0] - side).abs() < 1e-3, "{stated:?}");
    assert!((stated[3] - stated[1] - side).abs() < 1e-3, "{stated:?}");
}

/// The number of the indirect object whose body holds `needle` — an unfiltered stream `attach`
/// wrote, found by its bytes.
fn object_number_holding(bytes: &[u8], needle: &[u8]) -> u32 {
    let at = bytes
        .windows(needle.len())
        .position(|w| w == needle)
        .expect("the payload is in the file");
    let head = &bytes[..at];
    let obj = head
        .windows(4)
        .rposition(|w| w == b" obj")
        .expect("an object header");
    let line_start = head[..obj]
        .iter()
        .rposition(|&c| c == b'\n')
        .map_or(0, |at| at.saturating_add(1));
    std::str::from_utf8(&head[line_start..obj])
        .expect("ascii")
        .split(' ')
        .next()
        .and_then(|n| n.parse::<u32>().ok())
        .expect("an object number")
}

/// `--remove`: the entry gone from the tree, the other entries as they were, every byte of the
/// source under the update, and the objects the entry alone reached marked free — §7.5.6's
/// "shall be marked as deleted by means of their cross-reference entries" — with the generation
/// §7.5.4 gives an entry outside the linked list.
#[test]
fn a_file_removed_is_gone_from_the_tree_and_its_objects_are_marked_free_in_place() {
    let source = std::fs::read(committed("PDF20_AN001-BPC.pdf")).expect("a committed document");
    let once = attach(&source, b"first", "a.txt", None, None).expect("attached");
    let twice = attach(&once, b"second", "b.txt", None, None).expect("attached");
    let before = Document::open(twice.clone()).expect("opens");
    assert_eq!(tree_names(&before), ["a.txt", "b.txt"]);
    let a = attachments(&before)
        .into_iter()
        .find(|file| file.name == "a.txt")
        .expect("a.txt");

    let remove = |bytes: &[u8], name: &str| -> Result<Vec<u8>, Refusal> {
        let sinks = MemorySinks::new();
        let report = apply(
            &Plan::Attachments(AttachmentsPlan {
                source: 0,
                action: Action::Remove {
                    name: name.to_owned(),
                    names: "out.pdf".parse().expect("a pattern"),
                },
            }),
            &[Source::new(bytes.to_vec())],
            &sinks,
            &Policy::default(),
            &Budget::default(),
        )?;
        assert_eq!(report.exit(false, false), Exit::Success, "{report:?}");
        Ok(sinks.into_outputs().remove(0).1)
    };
    let removed = remove(&twice, "a.txt").expect("removed");
    assert_eq!(
        &removed[..twice.len()],
        &twice[..],
        "every byte of the source stays"
    );

    let after = Document::open(removed.clone()).expect("opens");
    assert_eq!(tree_names(&after), ["b.txt"]);
    let b = attachments(&after).into_iter().next().expect("b.txt");
    assert_eq!(
        after.decoded_stream_data(&b.stream).expect("decodes")[..],
        b"second"[..],
        "the other entry is untouched"
    );
    // The removed file's stream is still in the file — its bytes are under the update — and
    // the newest cross-reference section says its number names nothing now.
    let stream_id = object_number_holding(&twice, b"stream\nfirst\nendstream");
    assert!(before.xref().location(stream_id).is_some());
    assert!(
        after.xref().location(stream_id).is_none(),
        "object {stream_id} is free in the newest section"
    );
    assert!(
        after
            .xref()
            .object_numbers()
            .all(|number| number != stream_id),
        "a freed number is not an object"
    );
    assert!(
        a.stream.data.windows(5).any(|w| w == b"first"),
        "and the bytes are still in the file, under the update"
    );
    // The section's form is the file's own; the table form's free line is §7.5.4's, and the
    // stream form's type-0 entry is what `crates/pdf-syntax/tests/incremental_update.rs` holds
    // to Table 18 — here the reader's answer above is the statement.
    let tail = String::from_utf8_lossy(&removed[twice.len()..]);
    if tail.contains("trailer") {
        assert!(
            tail.contains("0000000000 65535 f"),
            "§7.5.4's free entry outside the linked list: {tail}"
        );
    }

    match remove(&removed, "a.txt") {
        Err(Refusal::NoSuchAttachment { name, .. }) => assert_eq!(name, "a.txt"),
        other => panic!("a name the tree no longer holds is refused: {other:?}"),
    }

    let dir = std::env::temp_dir().join(format!(
        "pdf-transform-writer-{}-removed",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("writable");
    let path = dir.join("removed.pdf");
    std::fs::write(&path, &removed).expect("written");
    if let Some(accepted) = qpdf_accepts(&path) {
        assert!(accepted, "qpdf --check does not accept the update");
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
