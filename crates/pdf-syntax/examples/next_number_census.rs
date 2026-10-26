//! Which documents a new object's number moves for, when it is counted from what the sections
//! stated rather than from what the reader kept — asked of the documents.
//!
//! ISO 32000-2 §7.5.4 gives a subsection a header stating "the number of entries in the
//! subsection", and the reader keeps fewer: a free entry names no object, and an entry that does
//! not lex is declared and unread. [`pdf_syntax::XrefTable::next_object_number`] numbers past every
//! stated number and past Table 15's `/Size` (ADR 2362). This prints, for every document, the two
//! rules the writers used before it — the larger of the highest kept number plus one and `/Size`
//! for the four incremental writers, the highest kept number plus one for the archive conversion —
//! beside the new answer, and names each document where one of them differs, with what the
//! sections stated that the reader did not keep.
//!
//! ```sh
//! cargo run --profile gates -p pdf-syntax --example next_number_census -- <file-or-directory>…
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::path::PathBuf;

use pdf_syntax::{Document, Object, ObjectId};

/// Every `.pdf` file the arguments name, a directory read one level deep.
fn files(arguments: impl Iterator<Item = String>) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for argument in arguments {
        let path = PathBuf::from(argument);
        if path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&path) {
                found.extend(
                    entries
                        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                        .filter(|path| path.extension().is_some_and(|ext| ext == "pdf")),
                );
            }
        } else {
            found.push(path);
        }
    }
    found.sort();
    found
}

fn main() {
    let mut opened = 0_usize;
    let mut unopened = 0_usize;
    let mut moved_update = 0_usize;
    let mut moved_archive = 0_usize;
    let mut moved_rebuilt = 0_usize;
    for path in files(std::env::args().skip(1)) {
        let Ok(bytes) = std::fs::read(&path) else {
            unopened = unopened.saturating_add(1);
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            unopened = unopened.saturating_add(1);
            continue;
        };
        opened = opened.saturating_add(1);
        let xref = document.xref();
        let kept = xref.object_numbers().max().unwrap_or_default();
        let size = document
            .trailer()
            .get("Size")
            .and_then(Object::as_integer)
            .and_then(|size| u32::try_from(size).ok())
            .unwrap_or_default();
        let update_before = kept.saturating_add(1).max(size);
        let archive_before = kept.saturating_add(1);
        let after = xref.next_object_number();
        let stated = xref.highest_stated_number().unwrap_or_default();
        let unread = (kept.saturating_add(1)..=stated)
            .filter(|number| xref.declared_and_unread(*number))
            .count();
        // What a writer numbering from the kept numbers could have landed on: a number a section
        // stated whose object this reader still resolves.
        let live = (kept.saturating_add(1)..=stated)
            .filter(|number| document.get(ObjectId::new(*number, 0)) != Object::Null)
            .count();
        let update_moved = update_before != after;
        let archive_moved = archive_before != after;
        if update_moved {
            moved_update = moved_update.saturating_add(1);
            if document.was_recovered() {
                moved_rebuilt = moved_rebuilt.saturating_add(1);
            }
        }
        if archive_moved {
            moved_archive = moved_archive.saturating_add(1);
        }
        if update_moved || archive_moved {
            let name = path
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
            println!(
                "{name}\tkept {kept}\tsize {size}\tstated {stated}\tunread above kept {unread}\tresolving above kept {live}\t\
                 rebuilt {}\tupdate {update_before} -> {after}\tarchive {archive_before} -> {after}",
                document.was_recovered()
            );
        }
    }
    println!(
        "opened {opened}, not opened {unopened}; the incremental writers' number moves for \
         {moved_update} ({moved_rebuilt} of them rebuilt, which those writers refuse), the \
         conversion's for {moved_archive}"
    );
}
