//! ISO 32000-2 §12.8's signatures, over the corpus.
//!
//! Eight documents carry one, and the interesting number is not how many but **what their byte
//! ranges cover**. §12.8.1 says a byte range digest "should be the entire PDF file, including the
//! signature dictionary but excluding the signature value itself", so a range that stops short
//! names bytes nobody signed — and measuring that needs no cryptography, only the file's length.
//!
//! This is the whole of what a renderer can say about a signature on its own evidence, and the
//! test exists to keep it honest in both directions: it is *not* a validity verdict, and a
//! document whose range covers everything is not thereby verified.

use std::path::{Path, PathBuf};

use pdf_model::signature::{Coverage, Right, UsageRights, permissions, signatures};
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

/// What a document's `/UR3` grants of the two rights this program can exceed.
///
/// §12.8.2.3's `should` binds a processor that writes, and what it binds is "modif[ying] a PDF …
/// in excess of the rights that are granted". This program fills in a field and saves, so those
/// are the two to ask about — and the point of printing them is trap 11's: the condition is
/// derived from the clause and then *counted*, rather than assumed to have members.
fn granted(name: &str, rights: &UsageRights) -> String {
    format!(
        "{name}: FillIn {}, FullSave {}, /P {}, /V 2.2 {}",
        rights.grants(Right::FillInForm),
        rights.grants(Right::FullSave),
        rights.restrictive,
        rights.version_understood
    )
}

/// Whether filling in a field and saving would exceed what this document grants.
fn exceeded(rights: &UsageRights) -> bool {
    !rights.grants(Right::FillInForm) || !rights.grants(Right::FullSave)
}

/// §12.8.2.3's condition, counted over the corpus rather than assumed to have members.
///
/// > A PDF processor that modifies a PDF, with a UR signature in excess of the rights that are
/// > granted by that signature, should remove that signature prior to writing the newly modified
/// > PDF.
///
/// **It has no members, and that is the finding rather than a gap.** All four documents carrying
/// a `/UR3` grant `/Form /FillIn` and `/Document /FullSave`, which is exactly what this program
/// does to a document, and all four come out `/P false` — two say so, two leave it to Table
/// 258's default — which is the entry that says "any possible restriction may be ignored", so
/// the arrays are not even reached. So `ViewState::save` withdraws no
/// signature on any file this corpus holds, and `usage_rights_are_withdrawn_when_a_save_exceeds
/// _them` in `forms_data.rs` is what exercises the code, on a file written for it. Held at zero
/// here so that a document arriving which *does* trip it announces itself. ADR 0159.
#[test]
fn no_corpus_documents_usage_rights_are_exceeded_by_what_this_program_does() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let mut stated = Vec::new();
    let mut withdrawn = Vec::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let Some(rights) = permissions(&document).usage_rights else {
            continue;
        };
        stated.push(granted(&name, &rights));
        if exceeded(&rights) {
            withdrawn.push(name.to_string());
        }
    }

    println!("§12.8.2.3's usage rights, and what they grant this program:");
    for entry in &stated {
        println!("  {entry}");
    }
    assert_eq!(stated.len(), 4, "documents with a /UR3: {stated:?}");
    assert!(
        withdrawn.is_empty(),
        "no corpus document's usage rights are exceeded by filling in a field and saving: \
         {withdrawn:?}"
    );
}

/// Every corpus signature, with what its range covers and what its document permits.
#[test]
fn every_signed_corpus_document_says_what_it_signed() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let mut signed = Vec::new();
    let mut certifications = 0usize;
    let mut with_permissions = Vec::new();
    let mut whole_file = 0usize;
    let mut unsigned_tails = Vec::new();
    let mut malformed = Vec::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let length = bytes.len() as u64;
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let found = signatures(&document);
        let perms = permissions(&document);
        if perms.doc_mdp.is_some() || perms.usage_rights.is_some() {
            with_permissions.push(format!("{name}: {perms:?}"));
        }
        if found.is_empty() {
            continue;
        }
        signed.push(format!(
            "{name}: {} signature(s), {:?}",
            found.len(),
            found
                .iter()
                .map(|signature| signature.sub_filter.clone().unwrap_or_default())
                .collect::<Vec<_>>()
        ));
        for signature in &found {
            certifications = certifications.saturating_add(usize::from(signature.certification));
            match signature.coverage(length) {
                Coverage::WholeFile => whole_file = whole_file.saturating_add(1),
                Coverage::Unsigned { tail } => {
                    unsigned_tails.push(format!("{name}: {tail} bytes after the signed range"));
                }
                Coverage::Malformed => malformed.push(format!(
                    "{name}: {:?} against {length} bytes",
                    signature.byte_range
                )),
            }
        }
    }

    println!("{} documents carry a signature:", signed.len());
    for entry in &signed {
        println!("  {entry}");
    }
    println!("  {certifications} of them are certification signatures (a DocMDP transform)");
    println!("  {whole_file} ranges cover the whole file");
    println!("  ranges with an unsigned tail: {unsigned_tails:?}");
    println!("  ranges that do not describe this file: {malformed:?}");
    println!("documents stating §12.8.6 permissions: {with_permissions:?}");
    assert_eq!(
        signed.len(),
        6,
        "documents with a signature in a signature field"
    );
    assert!(
        malformed.is_empty(),
        "every corpus signature's range describes its own file: {malformed:?}"
    );
    assert_eq!(whole_file, 4, "ranges running to the end of their file");

    // The two with an unsigned tail, and the second is §12.8.2.2 demonstrated by a real file.
    // `xfa_filled_imm1344e.pdf` is the corpus's one certification signature: its `/Perms`
    // `/DocMDP` states `/P 2`, which "permit[s] modifications that are appropriate for form
    // field or comment workflows", and 2.5 MB were appended after it was signed — a filled-in
    // form, saved by incremental update exactly as §12.8.1's NOTE 1 describes. Whether those
    // bytes contain *only* permitted changes is §12.8.2.2.2's question and needs the digest and
    // a comparison of two revisions; what this reader can say is that they are there.
    assert_eq!(
        unsigned_tails.len(),
        2,
        "signatures with bytes after them: {unsigned_tails:?}"
    );
    assert!(
        unsigned_tails
            .iter()
            .any(|entry| entry.starts_with("xfa_filled_imm1344e.pdf: 2542822")),
        "{unsigned_tails:?}"
    );
    assert_eq!(
        certifications, 1,
        "certification signatures, which §12.8.1 permits at most one of per document"
    );
    assert_eq!(
        with_permissions.len(),
        4,
        "documents stating §12.8.6 permissions: {with_permissions:?}"
    );
}
