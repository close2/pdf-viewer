//! ISO 32000-2 §12.3.5's collections, measured against the corpus.
//!
//! Like §12.4.3's articles, this family has **no witness in pdf.js**: not one of the 974
//! documents states a `/Collection`, a `/Folders`, or a `/RF` related files array. So the reader
//! is written from the clause and its two worked examples, and this file is a ratchet on that
//! corpus — the number that changes when a document with a portable collection arrives.
//!
//! **And, like §12.4.3's articles, this file used to say "no corpus witness at all", which is a
//! different and false claim**: `doc/corpora/format-corpus/pdfCabinetOfHorrors/`'s
//! `digitally_signed_3D_Portfolio.pdf` states a collection with eight schema fields and a
//! `/Folders` tree. The population a claim is about has to be the population it was measured
//! over, and this one was measured over the narrower of two. ADR 0405.
//!
//! The one thing it can still check over real files is the *naming convention*: §12.3.5.2 puts a
//! folder identifier inside an `/EmbeddedFiles` key, and every key in the corpus is measured
//! against it. All 23 are plain names, which is what the clause says a document without folders
//! writes.

#![expect(
    clippy::panic,
    reason = "test code: an optional corpus that is present but does not open is a broken \
              checkout, and must fail loudly rather than skip"
)]

use std::path::{Path, PathBuf};

use pdf_model::collection::{Collection, embedded_file_keys, folder_of, is_file_name};
use pdf_syntax::{Document, Object, ObjectId};

/// A `doc/corpora/format-corpus` document, or `None` when that optional submodule is not there.
///
/// Absent is a skip and present-but-unopenable is a panic, which is `doc/habits.md`'s rule.
fn format_corpus_document(relative: &str) -> Option<Document> {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../doc/corpora/format-corpus")
        .join(relative);
    let bytes = std::fs::read(path).ok()?;
    Some(Document::open(bytes).unwrap_or_else(|e| panic!("{relative} does not open: {e}")))
}

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
fn no_pdfjs_document_is_a_portable_collection() {
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
        "no pdf.js document has folders, so every key there is a plain name"
    );
    assert!(
        invalid_names.is_empty(),
        "keys §12.3.5.2 would not accept as file names: {invalid_names:?}"
    );
}

/// The corpus's one portable collection, from a producer rather than from this file.
///
/// `digitally_signed_3D_Portfolio.pdf` is what §12.3.5 is for and what this family had never been
/// shown: a `/Collection` with a schema, a `/Folders` tree, and the embedded files filed under it.
/// Until the five-hundred-and-seventieth session five places in this tree said no corpus document
/// stated one, on a count taken over pdf.js alone (ADR 0405).
///
/// **What it checks is the entry a hand-built fixture is weakest on**: §12.3.5.2's folder
/// identifiers inside `/EmbeddedFiles` keys. `folder_of` splits a key into a folder number and a
/// file name, and a fixture written beside the reader agrees with the reader by construction —
/// a producer's own keys do not.
#[test]
fn a_producers_own_portable_collection_is_read_with_its_folders() {
    let Some(document) =
        format_corpus_document("pdfCabinetOfHorrors/digitally_signed_3D_Portfolio.pdf")
    else {
        println!("skipped: doc/corpora/format-corpus is not checked out");
        return;
    };
    let collection = Collection::read(&document).expect("the catalog states a /Collection");
    let keys = embedded_file_keys(&document);
    let filed: Vec<(u32, &str)> = keys.iter().filter_map(|key| folder_of(key)).collect();
    println!(
        "schema {:?}, view {:?}, folders {}, keys {keys:?}",
        collection.schema.keys().collect::<Vec<_>>(),
        collection.view,
        collection.all_folders().len(),
    );
    println!("keys naming a folder: {filed:?}");

    assert_eq!(
        collection.schema.len(),
        8,
        "Table 155's columns this producer states: {:?}",
        collection.schema.keys().collect::<Vec<_>>()
    );
    assert!(
        collection.folders.is_some(),
        "the file this row exists for states a /Folders tree"
    );
    assert!(
        !keys.is_empty(),
        "a collection with no embedded files would arrange nothing"
    );
    assert!(
        keys.iter()
            .all(|key| is_file_name(key) || folder_of(key).is_some()),
        "every key is either a plain file name or a §12.3.5.2 folder identifier: {keys:?}"
    );
}
