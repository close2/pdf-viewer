//! Table 147 defaults at the Rust Qt bridge; no `QPrinter` or native dialog is driven.

use super::*;

#[test]
fn a_pdf_initializes_qt_print_job_fields_and_keeps_all_range_pairs() {
    let directory =
        std::env::temp_dir().join(format!("quorra-qt-print-defaults-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("temporary directory");
    let path = directory.join("stated-print-defaults.pdf");
    let bytes = written(&[
        "<< /Type /Catalog /Pages 2 0 R /ViewerPreferences << \
         /Duplex /DuplexFlipLongEdge /PrintScaling /None \
         /PrintPageRange [2 2 3 3] /NumCopies 4 >> >>",
        "<< /Type /Pages /Count 3 /Kids [3 0 R 4 0 R 5 0 R] >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Resources << >> >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Resources << >> >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Resources << >> >>",
    ]);
    let checked = pdf_syntax::Document::open(bytes.clone()).expect("exact-xref PDF opens");
    assert!(
        !checked.was_recovered(),
        "the preferences have real xref entries"
    );
    std::fs::write(&path, bytes).expect("write exact-xref preference fixture");
    let mut host = opened(&path);
    std::fs::remove_dir_all(&directory).expect("remove fixture after open");

    host.showing_what_would_print(3, viewer_core::Fidelity::Faithful, false);
    let job = host.print_job();
    assert_eq!(job.pages, 3);
    assert_eq!(job.copies, 4, "Table 147 /NumCopies");
    assert_eq!(
        job.duplex, 1,
        "PDF DuplexFlipLongEdge crosses as bridge long-edge value"
    );
    assert!(job.no_scaling, "PDF /PrintScaling /None");
    assert_eq!(
        job.page_ranges,
        vec![2, 2, 3, 3],
        "Table 147's two inclusive 1-based pairs, preserved for QPageRanges"
    );
}
