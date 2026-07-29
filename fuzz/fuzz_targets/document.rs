//! Fuzzes opening a whole document.
//!
//! The widest target: cross-reference parsing, the scan-based recovery path, object stream
//! expansion and filter decoding all run here. It is also the one that mirrors what a
//! viewer actually does with a file it was handed, so a crash here is a crash on opening
//! an attachment.
//!
//! Traversal is included because reaching the catalogue and the first page is where cycles
//! in the object graph bite, and a cycle that loops forever is as bad as a panic.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pdf_syntax::{Document, Limits, Object};

fuzz_target!(|data: &[u8]| {
    let limits = Limits {
        max_depth: 32,
        max_array_len: 4096,
        max_dict_len: 256,
        max_string_len: 1 << 16,
        max_stream_len: 1 << 22,
    };

    let Ok(document) = Document::open_with_limits(data.to_vec(), limits) else {
        return;
    };

    // Opening is not enough: the object graph must also be safe to walk.
    let Ok(catalog) = document.catalog() else {
        return;
    };

    let pages = document.get_key(&catalog, "Pages");
    let Some(pages) = pages.as_dict() else { return };

    // Descend the page tree. A `/Kids` cycle must terminate by the bound rather than by
    // exhausting memory.
    let mut node = pages.clone();
    for _ in 0..16 {
        let kids = document.get_key(&node, "Kids");
        let Some(kids) = kids.as_array() else { break };
        let Some(first) = kids.first() else { break };
        let child = document.resolve(first);
        let Some(child) = child.as_dict() else { break };
        node = child.clone();
    }

    // And decode whatever content the leaf claims to have.
    match document.get_key(&node, "Contents") {
        Object::Stream(stream) => {
            let _ = document.decoded_stream_data(&stream);
        }
        Object::Array(items) => {
            for item in items.iter().take(16) {
                if let Some(stream) = document.resolve(item).as_stream() {
                    let _ = document.decoded_stream_data(stream);
                }
            }
        }
        _ => {}
    }
});
