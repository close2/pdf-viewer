//! How many font dictionaries state a `/ToUnicode` that is not the stream the clause requires.
//!
//! The instrument behind ADR 1027. ISO 32000-2 §9.10.1 says of the entry that its
//!
//! > value shall be a stream object containing a special kind of CMap file that maps character
//! > codes to Unicode values
//!
//! and Table 119 (Type 0), Table 109 (Type 1), Table 110 (Type 3) and Table 117 all type it
//! `stream`. A value of any other kind is therefore a `shall` the file broke, and this crate's
//! reader answers it the only way it can — [`pdf_font`]'s `read_to_unicode` asks for a stream and
//! takes nothing from a name. What that cost until session 1008 was a *report*: a composite font
//! refused for §9.7.5.2 said "it states no `/ToUnicode`", which is a claim about the file and is
//! false of every document this census names.
//!
//! ```sh
//! cargo run --release -p pdf-font --example to_unicode_kind_census -- doc/pdf.js doc/corpora
//! ```
//!
//! **Every object the cross-reference table names and every dictionary nested inside one** is
//! walked, for trap 25's reason: a font dictionary need not be an indirect object, and
//! `issue11555.pdf` writes its whole `Type0` inline in a page's `/Resources`. What is counted is
//! the *dictionary*, not the page: which fonts a page shows is a content stream's business and
//! this crate reads none.
//!
//! The population is a property of the files and is the same on every machine, so a number off
//! this census is comparable between rounds without re-running it — which is exactly what a
//! number off a *render* is not.

#![expect(
    clippy::print_stdout,
    reason = "an example whose whole product is a census"
)]

use std::collections::BTreeMap;

use pdf_font::tounicode::ToUnicode;
use pdf_syntax::{Dictionary, Document, FileBytes, Object, ObjectId};

fn main() {
    let roots: Vec<String> = std::env::args().skip(1).collect();
    let roots = if roots.is_empty() {
        vec!["doc".to_owned()]
    } else {
        roots
    };
    let mut files = Vec::new();
    for root in &roots {
        collect(std::path::Path::new(root), &mut files);
    }
    files.sort();

    let mut documents = 0usize;
    let mut fonts = 0usize;
    let mut with_entry = 0usize;
    let mut as_stream = 0usize;
    // A stream that decodes and states no `bf` mapping at all — `read_to_unicode` answers empty
    // for it exactly as it does for a name, and the refusal used to call both of them absent.
    let mut empty_streams: BTreeMap<String, usize> = BTreeMap::new();
    // Keyed by the kind, and for a name by the name itself, because *which* name a producer
    // wrote is the whole of what it was trying to say.
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    let mut named_documents: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();

    for path in &files {
        let Ok(bytes) = FileBytes::on_disk(path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        documents = documents.saturating_add(1);
        let mut found = Vec::new();
        for number in document.xref().object_numbers() {
            let object = document.get(ObjectId::new(number, 0));
            collect_fonts(&document, &object, &mut found);
        }
        for dict in found {
            fonts = fonts.saturating_add(1);
            let object = document.get_key(&dict, "ToUnicode");
            if matches!(object, Object::Null) {
                continue;
            }
            with_entry = with_entry.saturating_add(1);
            if let Some(stream) = object.as_stream() {
                as_stream = as_stream.saturating_add(1);
                let mapped = document
                    .decoded_stream_data(stream)
                    .is_some_and(|bytes| !ToUnicode::parse(&bytes).is_empty());
                if !mapped {
                    let name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    let found: &mut usize = empty_streams.entry(name).or_default();
                    *found = found.saturating_add(1);
                }
                continue;
            }
            let kind = kind_of(&object);
            let count: &mut usize = kinds.entry(kind.clone()).or_default();
            *count = count.saturating_add(1);
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let found: &mut usize = named_documents
                .entry(kind)
                .or_default()
                .entry(name)
                .or_default();
            *found = found.saturating_add(1);
        }
    }

    let wrong: usize = kinds.values().copied().fold(0usize, usize::saturating_add);
    println!("{} files, {documents} opened", files.len());
    println!(
        "{fonts} font dictionaries: {with_entry} state a /ToUnicode, {as_stream} of those as the \
         stream §9.10.1 requires and {wrong} as something else"
    );
    for (kind, count) in &kinds {
        println!("  {kind}: {count} font dictionary/ies");
        for (name, found) in named_documents.get(kind).into_iter().flatten() {
            println!("    {name}: {found}");
        }
    }
    let empty: usize = empty_streams
        .values()
        .copied()
        .fold(0usize, usize::saturating_add);
    println!(
        "and {empty} of the {as_stream} streams state no bf mapping at all, over {} documents — \
         which `read_to_unicode` answers empty for exactly as it does a name",
        empty_streams.len()
    );
    for (name, found) in &empty_streams {
        println!("    {name}: {found}");
    }
}

/// The value's kind, spelled the way the refusal spells it.
fn kind_of(object: &Object) -> String {
    match object {
        Object::Name(name) => format!("the name /{}", String::from_utf8_lossy(name.as_bytes())),
        Object::Array(_) => "an array".to_owned(),
        Object::Dictionary(_) => "a dictionary".to_owned(),
        Object::String(_) => "a string".to_owned(),
        Object::Boolean(_) => "a boolean".to_owned(),
        Object::Integer(_) | Object::Real(_) => "a number".to_owned(),
        Object::Null | Object::Stream(_) | Object::Reference(_) => "not counted here".to_owned(),
    }
}

/// Every font dictionary inside one object, itself included.
///
/// `/Type /Font` rather than a `/Subtype` list: Table 109, Table 110, Table 117 and Table 119 all
/// carry the entry, so narrowing to one of them would measure a subset of the clause.
fn collect_fonts(document: &Document, object: &Object, into: &mut Vec<Dictionary>) {
    match object {
        Object::Dictionary(dict) => {
            if document
                .get_key(dict, "Type")
                .as_name()
                .is_some_and(|name| name.as_bytes() == b"Font")
            {
                into.push(dict.clone());
            }
            for (_, value) in dict.iter() {
                collect_fonts(document, value, into);
            }
        }
        Object::Stream(stream) => {
            for (_, value) in stream.dict.iter() {
                collect_fonts(document, value, into);
            }
        }
        Object::Array(items) => {
            for item in items {
                collect_fonts(document, item, into);
            }
        }
        _ => {}
    }
}

/// Every `.pdf` under `dir`, recursively.
fn collect(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "pdf") {
            out.push(path);
        }
    }
}
