//! Which corpus documents state a `Lab` space, and under which white point.
//!
//! ISO 32000-2 Table 64 makes a `Lab` space's `/WhitePoint` required, and §8.6.5.4's second
//! transformation stage multiplies by it; `ColourSpace::Lab` carries it and adapts it onto D50
//! (ADR 1712). Every document this prints with a white point other than the connection space's D50
//! is one whose `Lab` colours that change moves, and the rest are not — so this is the command that
//! names the pages a corpus arm may see move, and the number is not written down anywhere else.
//!
//! Every object the cross-reference table lists is scanned, rather than every space a page reaches,
//! as `pdf-model`'s `black_point_census` does: a `Lab` space inside an `/Indexed` base or a
//! `/DeviceN` alternate is the same statement as one in a page's own `/ColorSpace`. An encrypted
//! document is opened with the password `pdf-model`'s `corpus_passwords` publishes for it, as the
//! corpus gates open it, and every document that still will not open is named: a census that
//! skipped one in silence would say nothing about the pages it holds.
//!
//! ```sh
//! cargo run --release -p pdf-colour --example lab_white_point_census
//! ```
#![expect(
    clippy::print_stdout,
    clippy::expect_used,
    reason = "a measurement whose output is its purpose, and which stops loudly where the corpus \
              is missing"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pdf_syntax::{Document, Limits, Object, ObjectId, SyntaxError};

#[path = "../../pdf-model/tests/support/corpus_passwords.rs"]
#[expect(
    dead_code,
    reason = "the references' spelling of a password is `pdf-model`'s oracle's; this census \
              opens documents and hands a password to nothing else"
)]
mod corpus_passwords;

/// How far an object is followed into the objects it holds directly.
const MAX_DEPTH: usize = 8;

/// The connection space's white, ICC.1's PCS illuminant, which is the white `Lab` was read under
/// before its own was carried.
const D50: [f64; 3] = [0.9642, 1.0, 0.8249];

fn corpus() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../doc/pdf.js/test/pdfs");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&root)
        .expect("the submodule is checked out")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|e| e == "pdf"))
        .collect();
    files.sort();
    files
}

/// A `/WhitePoint` as three numbers, or `None` where the entry is absent or is not three numbers.
fn white_point(document: &Document, dict: &pdf_syntax::Dictionary) -> Option<[f64; 3]> {
    let Object::Array(items) = document.get_key(dict, "WhitePoint") else {
        return None;
    };
    let values: Vec<f64> = items
        .iter()
        .filter_map(|item| document.resolve(item).as_number())
        .collect();
    <[f64; 3]>::try_from(values.as_slice()).ok()
}

/// Every `Lab` space inside one object, its white point written as the file writes it.
fn scan(document: &Document, object: &Object, depth: usize, found: &mut Vec<String>) {
    if depth > MAX_DEPTH {
        return;
    }
    match object {
        Object::Array(items) => {
            if let Some(Object::Name(head)) = items.first()
                && head.as_bytes() == b"Lab"
            {
                let dict = items.get(1).map(|d| document.resolve(d));
                found.push(
                    match dict
                        .as_ref()
                        .and_then(Object::as_dict)
                        .and_then(|dict| white_point(document, dict))
                    {
                        Some(white) => format!("{white:?}"),
                        None => "no WhitePoint".to_owned(),
                    },
                );
            }
            for item in items {
                scan(document, item, depth.saturating_add(1), found);
            }
        }
        Object::Dictionary(dict) => {
            for (_, value) in dict.iter() {
                scan(document, value, depth.saturating_add(1), found);
            }
        }
        Object::Stream(stream) => {
            for (_, value) in stream.dict.iter() {
                scan(document, value, depth.saturating_add(1), found);
            }
        }
        _ => {}
    }
}

/// Whether a white point as printed is the connection space's D50, to the four places PDF
/// writers print it.
fn is_d50(shown: &str) -> bool {
    let numbers: Vec<f64> = shown
        .trim_matches(|c| c == '[' || c == ']')
        .split(", ")
        .filter_map(|value| value.parse().ok())
        .collect();
    numbers.len() == 3
        && numbers
            .iter()
            .zip(D50)
            .all(|(value, d50)| (value - d50).abs() < 1e-3)
}

fn main() {
    let mut documents = 0_usize;
    let mut unopened: Vec<String> = Vec::new();
    let mut stating: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for path in corpus() {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let opened = match (
            Document::open(bytes.clone()),
            corpus_passwords::corpus_password(&name),
        ) {
            (Err(SyntaxError::PasswordRequired), Some(known)) => {
                Document::open_with_password(bytes, Limits::default(), known.password)
            }
            (opened, _) => opened,
        };
        let document = match opened {
            Ok(document) => document,
            Err(why) => {
                unopened.push(format!("{name}: {why}"));
                continue;
            }
        };
        documents = documents.saturating_add(1);
        let mut found = Vec::new();
        let numbers: Vec<u32> = document.xref().object_numbers().collect();
        for number in numbers {
            let object = document.get(ObjectId::new(number, 0));
            scan(&document, &object, 0, &mut found);
        }
        for white in found {
            let counter = stating
                .entry(name.clone())
                .or_default()
                .entry(white)
                .or_default();
            *counter = counter.saturating_add(1);
        }
    }
    let other: Vec<_> = stating
        .iter()
        .filter(|(_, whites)| whites.keys().any(|white| !is_d50(white)))
        .collect();
    println!("{documents} document(s) opened, {} not:", unopened.len());
    for why in &unopened {
        println!("    {why}");
    }
    println!("  {} state a Lab space:", stating.len());
    for (file, whites) in &stating {
        println!("    {file}  {whites:?}");
    }
    println!("  {} state one whose white point is not D50:", other.len());
    for (file, whites) in other {
        println!("    {file}  {whites:?}");
    }
}
