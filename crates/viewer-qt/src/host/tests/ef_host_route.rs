//! Annex O: the real host opens an embedded PDF as a second tab and applies its fragment.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "the test builds small deterministic PDFs and fails loudly on invalid fixtures"
)]

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use super::super::Host;
use super::drawn;
use pdf_model::view::WidgetAppearances;
use viewer_core::{Answer, Query};
use viewer_host::trace::Trace;
use viewer_host::{EmbeddedDocuments, ReaderWords, Settings, native_window::DOCUMENT};

static SCRATCH_ID: AtomicUsize = AtomicUsize::new(0);

/// Owns the temporary fixture directory and removes it after the host has finished reading it.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let serial = SCRATCH_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "pdf-viewer-ef-host-route-{}-{serial}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("create a unique fixture directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Writes a complete, uncompressed PDF with a classic xref table and the first object as Catalog.
fn assembled(objects: &[String]) -> Vec<u8> {
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, body) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        let object_number = index + 1;
        write!(bytes, "{object_number} 0 obj\n{body}\nendobj\n").expect("write into bytes");
    }

    let xref = bytes.len();
    let size = objects.len() + 1;
    write!(bytes, "xref\n0 {size}\n0000000000 65535 f \n").expect("write xref header");
    for offset in offsets {
        writeln!(bytes, "{offset:010} 00000 n ").expect("write xref entry");
    }
    write!(
        bytes,
        "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n"
    )
    .expect("write trailer");
    bytes
}

/// A valid two-page child PDF whose second page is the target of `page=2`.
fn child_pdf() -> Vec<u8> {
    let empty_content = "<< /Length 0 >>\nstream\n\nendstream".to_owned();
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Resources << >> /Contents 5 0 R >>"
            .to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Resources << >> /Contents 5 0 R >>"
            .to_owned(),
        empty_content,
    ])
}

/// A one-page holder with `target.pdf` in the Catalog's `EmbeddedFiles` name tree.
fn holder_pdf(child: &[u8]) -> Vec<u8> {
    let child_stream = String::from_utf8(child.to_vec()).expect("the child PDF fixture is ASCII");
    let embedded = format!(
        "<< /Type /EmbeddedFile /Subtype /application#2Fpdf /Length {} >>\nstream\n{}\nendstream",
        child.len(),
        child_stream
    );
    let empty_content = "<< /Length 0 >>\nstream\n\nendstream".to_owned();
    assembled(&[
        "<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles 4 0 R >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Resources << >> /Contents 7 0 R >>"
            .to_owned(),
        "<< /Names [(target.pdf) 5 0 R] >>".to_owned(),
        "<< /Type /Filespec /F (target.pdf) /UF (target.pdf) /EF << /F 6 0 R /UF 6 0 R >> >>"
            .to_owned(),
        embedded,
        empty_content,
    ])
}

/// Checks that fixture parsing uses the declared xref and that each page is interpretable.
fn assert_complete_pdf(bytes: &[u8], expected_pages: usize) {
    let document = pdf_syntax::Document::open(bytes.to_vec()).expect("fixture is a valid PDF");
    assert!(
        !document.was_recovered(),
        "the declared xref is used without repair"
    );
    for index in 0..expected_pages {
        let page = pdf_model::Pages::new(&document)
            .get(index)
            .expect("the fixture contains every declared page");
        assert!(
            pdf_model::interpret(&document, &page).is_complete(),
            "page {index} has complete interpretation"
        );
    }
    assert!(
        pdf_model::Pages::new(&document)
            .get(expected_pages)
            .is_none(),
        "the page tree contains exactly {expected_pages} pages"
    );
}

#[test]
fn ef_opens_extracted_pdf_as_second_tab_and_applies_remainder_to_child() {
    let child = child_pdf();
    let holder = holder_pdf(&child);
    assert_complete_pdf(&holder, 1);
    assert_complete_pdf(&child, 2);

    let scratch = Scratch::new();
    let path = scratch.path().join("holder.pdf");
    std::fs::write(&path, &holder).expect("write holder fixture");

    let settings = Settings {
        embedded_documents: EmbeddedDocuments::Open,
        ..Settings::default()
    };
    let mut host = Host::open(
        &path,
        Some("ef=target.pdf&page=2".to_owned()),
        WidgetAppearances::Delegated,
        settings,
        ReaderWords::default(),
        Trace::off(Instant::now()),
    )
    .expect("the complete holder opens");

    // Join the launch thread as the existing `opened_under` helper does. Extraction is handled
    // by the real Qt host pump; its open policy stores the child in Arrivals and raises this
    // deferred-arrival flag.
    host.resized(800, 1000, 1.0);
    drawn(&mut host);

    assert_eq!(host.documents(), ["holder.pdf".to_owned()]);
    assert_eq!(host.focused_document(), 0);
    let Answer::Page {
        document,
        index,
        of,
        ..
    } = host.state.viewer.query(Query::CurrentPage)
    else {
        panic!("the holder has a current page before the embedded open");
    };
    assert_eq!(document, DOCUMENT, "the containing document remains open");
    assert_eq!(index, 0);
    assert_eq!(of, 1, "the holder has one page before the second open");

    assert!(
        host.arrival_due(),
        "Event::Extracted reached the host and queued the embedded PDF"
    );
    host.arrive();
    drawn(&mut host);

    assert_eq!(
        host.documents(),
        ["holder.pdf".to_owned(), "target.pdf".to_owned()],
        "the holder retains its tab and the extracted PDF is a separate document"
    );
    assert_eq!(host.focused_document(), 1, "the embedded PDF is in front");
    let Answer::Page {
        document,
        index,
        of,
        ..
    } = host.state.viewer.query(Query::CurrentPage)
    else {
        panic!("the embedded document has a current page");
    };
    assert_eq!(document, viewer_core::DocumentId(2));
    assert_eq!(index, 1, "page=2 after ef applies to the embedded PDF");
    assert_eq!(of, 2, "the opened document is the two-page embedded PDF");
}
