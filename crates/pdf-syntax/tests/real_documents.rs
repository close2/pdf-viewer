//! Opens actual PDF files, including the specification itself.
//!
//! Unit tests on hand-written fragments confirm the parser does what its author expected.
//! These confirm it copes with what real producers emit — cross-reference streams, object
//! streams, compressed content, incremental updates — which is a different question and
//! the one that decides whether the crate is usable.
//!
//! The specification PDFs in `doc/` are good subjects precisely because nobody wrote them
//! with this parser in mind.

#![expect(
    clippy::expect_used,
    clippy::print_stdout,
    reason = "test code: an explanatory panic is the intended failure, and the survey \
              output is worth reading"
)]

use std::path::{Path, PathBuf};

use pdf_syntax::{Document, Object};

/// Returns the specification PDFs shipped in `doc/`.
fn corpus() -> Vec<PathBuf> {
    let doc = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&doc)
        .expect("doc/ is readable")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "pdf"))
        .collect();
    files.sort();
    files
}

/// The hand-written fixture, which `test-scenes` also renders.
#[test]
fn the_minimal_fixture_parses() {
    let document = Document::open(test_scenes::basic_pdf()).expect("the fixture is a valid PDF");

    assert!(
        !document.was_recovered(),
        "the fixture's xref table is correct and should be used"
    );

    let catalog = document.catalog().expect("the fixture has a catalogue");
    assert_eq!(
        document
            .get_key(&catalog, "Type")
            .as_name()
            .map(|name| name == &"Catalog"),
        Some(true)
    );

    let pages = document.get_key(&catalog, "Pages");
    let pages = pages.as_dict().expect("/Pages resolves to a dictionary");
    assert_eq!(document.get_key(pages, "Count").as_integer(), Some(1));

    // Reach the page and its content stream, which is the whole point of the crate.
    let kids = document.get_key(pages, "Kids");
    let kids = kids.as_array().expect("/Kids is an array");
    assert_eq!(kids.len(), 1);

    let page = document.resolve(&kids[0]);
    let page = page.as_dict().expect("the kid is a page dictionary");
    let contents = document.get_key(page, "Contents");
    let stream = contents.as_stream().expect("/Contents is a stream");
    let data = document
        .decoded_stream_data(stream)
        .expect("uncompressed data decodes");

    let text = String::from_utf8_lossy(&data);
    assert!(
        text.contains("100 100 200 200 re"),
        "the red rectangle should be there"
    );
    assert!(text.contains("W n"), "and the clip");
}

/// Every specification PDF must open, expose a catalogue and report a page count.
///
/// These files use cross-reference streams and object streams throughout, so this
/// exercises the PDF 1.5 path rather than the classic table.
#[test]
fn every_specification_pdf_opens() {
    let files = corpus();
    assert!(!files.is_empty(), "the corpus should not be empty");

    for path in &files {
        let bytes = std::fs::read(path).expect("corpus file is readable");
        let name = path.file_name().unwrap_or_default().to_string_lossy();

        let document =
            Document::open(bytes).unwrap_or_else(|e| panic!("{name} failed to open: {e}"));
        let catalog = document
            .catalog()
            .unwrap_or_else(|e| panic!("{name} has no usable catalogue: {e}"));

        let pages = document.get_key(&catalog, "Pages");
        let pages = pages
            .as_dict()
            .unwrap_or_else(|| panic!("{name}: /Pages is not a dictionary"));
        let count = document
            .get_key(pages, "Count")
            .as_integer()
            .unwrap_or_else(|| panic!("{name}: /Pages has no /Count"));

        assert!(count > 0, "{name} should have at least one page");
        println!(
            "{name}: {count} pages, {} objects{}",
            document.xref().len(),
            if document.was_recovered() {
                " (xref recovered by scan)"
            } else {
                ""
            }
        );
    }
}

/// Content streams must decode, which means the filter chain and object streams both work.
#[test]
fn the_first_page_content_stream_decodes() {
    for path in corpus() {
        let bytes = std::fs::read(&path).expect("readable");
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let document = Document::open(bytes).unwrap_or_else(|e| panic!("{name}: {e}"));

        let catalog = document.catalog().unwrap_or_else(|e| panic!("{name}: {e}"));
        let pages = document.get_key(&catalog, "Pages");
        let Some(pages) = pages.as_dict() else {
            continue;
        };

        // Walk down the left edge of the page tree to the first leaf.
        let mut node = pages.clone();
        for _ in 0..32 {
            let kids = document.get_key(&node, "Kids");
            let Some(kids) = kids.as_array() else { break };
            let Some(first) = kids.first() else { break };
            let child = document.resolve(first);
            let Some(child) = child.as_dict() else { break };
            node = child.clone();
        }

        let contents = document.get_key(&node, "Contents");
        // `/Contents` may be a single stream or an array of them.
        let streams: Vec<Object> = match &contents {
            Object::Array(items) => items.iter().map(|item| document.resolve(item)).collect(),
            other => vec![other.clone()],
        };

        let mut decoded_bytes = 0usize;
        for object in &streams {
            let Some(stream) = object.as_stream() else {
                continue;
            };
            let data = document
                .decoded_stream_data(stream)
                .unwrap_or_else(|| panic!("{name}: first page content stream did not decode"));
            decoded_bytes = decoded_bytes.saturating_add(data.len());
        }

        assert!(
            decoded_bytes > 0,
            "{name}: the first page decoded to nothing"
        );
        println!("{name}: first page content is {decoded_bytes} bytes decoded");
    }
}

/// A file whose cross-reference table is destroyed must still open, by scanning.
///
/// This is the recovery path that decides whether the reader is usable on real documents,
/// so it is tested with a deliberately broken file rather than trusted.
#[test]
fn a_corrupt_cross_reference_table_is_recovered_by_scanning() {
    let complete = test_scenes::basic_pdf();

    // Replace the trailing `startxref <offset> %%EOF` with one pointing at a wildly wrong
    // offset, as a truncated or carelessly-edited file would. Rebuilding the tail rather
    // than splicing digits in place keeps this independent of the fixture's exact layout.
    let at = complete
        .windows(b"startxref".len())
        .rposition(|window| window == b"startxref")
        .expect("the fixture has a startxref");

    let mut bytes = complete.get(..at).unwrap_or_default().to_vec();
    bytes.extend_from_slice(b"startxref\n999999\n%%EOF\n");

    let document = Document::open(bytes).expect("a broken xref should be recovered, not fatal");
    assert!(
        document.was_recovered(),
        "recovery should be reported, not hidden"
    );

    // And the recovered document must actually work.
    let catalog = document
        .catalog()
        .expect("the catalogue is findable by scanning");
    let pages = document.get_key(&catalog, "Pages");
    assert!(
        pages.as_dict().is_some(),
        "the page tree is reachable after recovery"
    );
}

/// Truncation is the most common corruption of all.
#[test]
fn a_truncated_file_does_not_hang_or_panic() {
    let complete = test_scenes::basic_pdf();

    // Every prefix, so every possible truncation point is covered.
    for length in 0..complete.len() {
        let truncated = complete.get(..length).unwrap_or_default().to_vec();
        // Success or failure are both acceptable; hanging or panicking are not.
        if let Ok(document) = Document::open(truncated) {
            // Whatever opened must be safe to interrogate.
            let _ = document.catalog();
        }
    }
}
