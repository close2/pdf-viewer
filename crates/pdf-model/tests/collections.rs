//! ISO 32000-2 §12.3.5's collections, measured against the corpus.
//!
//! Like §12.4.3's articles, this family has **no corpus witness at all**: not one of the 974
//! documents states a `/Collection`, a `/Folders`, or a `/RF` related files array. So the reader
//! is written from the clause and its two worked examples, and this file is a ratchet on the
//! corpus — the number that changes when a document with a portable collection arrives.
//!
//! The one thing it can still check over real files is the *naming convention*: §12.3.5.2 puts a
//! folder identifier inside an `/EmbeddedFiles` key, and every key in the corpus is measured
//! against it. All 23 are plain names, which is what the clause says a document without folders
//! writes.

use std::path::{Path, PathBuf};

use pdf_model::collection::{Collection, embedded_file_keys, folder_of, is_file_name};
use pdf_syntax::{Document, Object, ObjectId};

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

/// What the corpus holds of §12.3.5, §12.3.6 and §7.11.4.2, which is nothing but the keys.
#[test]
fn no_corpus_document_is_a_portable_collection() {
    let Some(files) = corpus() else {
        println!("skipped: the doc/pdf.js submodule is not checked out");
        return;
    };

    let mut collections = Vec::new();
    let mut related_files = Vec::new();
    let mut keys = 0usize;
    let mut in_a_folder = 0usize;
    let mut invalid_names = Vec::new();
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if Collection::read(&document).is_some() {
            collections.push(name.clone().into_owned());
        }
        for key in embedded_file_keys(&document) {
            keys = keys.saturating_add(1);
            match folder_of(&key) {
                Some((folder, file)) => {
                    in_a_folder = in_a_folder.saturating_add(1);
                    if !is_file_name(file) {
                        invalid_names.push(format!("{name}: <{folder}>{file}"));
                    }
                }
                None => {
                    if !is_file_name(&key) {
                        invalid_names.push(format!("{name}: {key}"));
                    }
                }
            }
        }

        // §7.11.4.2's `/RF` is on a file specification, which may sit anywhere — an annotation,
        // an action, the name tree — so this looks at every object rather than only at the
        // attachments.
        for number in document.xref().object_numbers() {
            let object = document.get(ObjectId {
                number,
                generation: 0,
            });
            let Some(dict) = object.as_dict() else {
                continue;
            };
            if dict.get("RF").is_some()
                && dict
                    .get("Type")
                    .and_then(Object::as_name)
                    .is_some_and(|kind| kind.as_bytes() == b"Filespec")
            {
                related_files.push(format!("{name} object {number}"));
            }
        }
    }

    println!("documents that are portable collections: {collections:?}");
    println!("file specifications with a /RF: {related_files:?}");
    println!("{keys} /EmbeddedFiles keys, {in_a_folder} naming a folder");
    println!("keys that are not valid file names: {invalid_names:?}");

    assert!(collections.is_empty(), "documents with a /Collection");
    assert!(related_files.is_empty(), "specifications with a /RF");
    assert_eq!(keys, 23, "embedded files across the corpus");
    assert_eq!(
        in_a_folder, 0,
        "no document has folders, so every key is a plain name"
    );
    assert!(
        invalid_names.is_empty(),
        "keys §12.3.5.2 would not accept as file names: {invalid_names:?}"
    );
}
