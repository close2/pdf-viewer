//! ISO 32000-2 §12.4.2's page labels, over the corpus and over the clause's own example.
//!
//! Two tests with two different jobs, which is trap 4 and trap 8 in one file. The corpus test
//! says the number tree is walked and the ranges are found in documents somebody else wrote;
//! the constructed test says the *labels* are the ones the clause's example states, which no
//! corpus document happens to exercise all of.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use pdf_model::page_label::PageLabels;
use pdf_syntax::Document;

/// The pdf.js corpus, or `None` when the submodule is not checked out.
fn corpus() -> Option<Vec<PathBuf>> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "pdf"))
        .collect();
    files.sort();
    Some(files)
}

/// Every document that states page labels produces one for its first page.
///
/// §12.4.2 says "[t]he tree shall include a value for page index 0". So a document with a
/// `/PageLabels` that answers nothing for page zero has either a tree this reader cannot walk
/// or a file that breaks that sentence, and the two are worth telling apart — which is why the
/// assertion is about the *count* of documents rather than about any one of them, and why the
/// failures are named.
#[test]
fn every_document_stating_labels_labels_its_first_page() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let mut stating = 0usize;
    let mut silent_at_zero = Vec::new();
    let mut examples = Vec::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let labels = PageLabels::read(&document);
        if labels.is_empty() {
            continue;
        }
        stating = stating.saturating_add(1);
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        match labels.label(0) {
            Some(label) => {
                if examples.len() < 8 {
                    let mut line = String::new();
                    let _ = write!(line, "{name}: ");
                    for index in 0..4 {
                        let _ = write!(line, "{:?} ", labels.label(index));
                    }
                    examples.push(line);
                }
                assert!(
                    label.len() < 1024,
                    "{name}: a page label of {} bytes is not a label",
                    label.len()
                );
            }
            None => silent_at_zero.push(name.into_owned()),
        }
    }

    println!("{stating} of {} documents state page labels", files.len());
    for line in &examples {
        println!("  {line}");
    }
    assert!(stating > 0, "the corpus has documents with /PageLabels");
    assert!(
        silent_at_zero.is_empty(),
        "these documents state labels and label no first page: {silent_at_zero:?}"
    );
}

/// §12.4.2's own worked example, built as a file.
///
/// > The following example shows a document with pages labelled i, ii, iii, iv, 1, 2, 3, A-8, A-
///
/// Three ranges: lowercase Roman from page 0, decimal from page 4, and decimal with the prefix
/// `A-` and `/St 8` from page 7. The corpus cannot check this — no document in it uses all
/// three forms — which is trap 8's argument, and the example is the one place the standard
/// states the answers rather than the rules.
#[test]
fn the_clauses_own_example_produces_the_labels_it_states() {
    let objects = [
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R /PageLabels 4 0 R >>\nendobj\n",
        "2 0 obj\n<< /Type /Pages /Count 1 /Kids [3 0 R] >>\nendobj\n",
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>\nendobj\n",
        "4 0 obj\n<< /Nums [0 << /S /r >> 4 << /S /D >> 7 << /S /D /P (A-) /St 8 >>] >>\nendobj\n",
    ];

    let mut out = String::from("%PDF-1.7\n");
    let mut offsets = Vec::new();
    for object in objects {
        offsets.push(out.len());
        out.push_str(object);
    }
    let xref_at = out.len();
    let _ = write!(out, "xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for offset in &offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_at}\n%%EOF\n",
        objects.len() + 1
    );

    let document = Document::open(out.into_bytes()).expect("a valid file");
    let labels = PageLabels::read(&document);
    let shown: Vec<String> = (0..9)
        .map(|index| labels.label(index).unwrap_or_default())
        .collect();

    assert_eq!(
        shown,
        ["i", "ii", "iii", "iv", "1", "2", "3", "A-8", "A-9"],
        "the clause states these nine labels for this tree"
    );
}
