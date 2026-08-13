//! How many corpus pages state a Cal space's `/BlackPoint`, and how many state a real one.
//!
//! ISO 32000-2 Table 62, Table 63 and Table 64 give a `CalGray`, a `CalRGB` and a `Lab` space an
//! optional `/BlackPoint`, "the tristimulus value, in the CIE 1931 XYZ space, of the diffuse black
//! point", defaulting to `[0.0 0.0 0.0]`. §8.6.5.3 says the entry "shall control the overall effect
//! of the CIE-based gamut mapping function described in subclause 10.3", and §8.6.5.9 leaves
//! whether black point compensation happens at all to the processor wherever `/UseBlackPtComp` is
//! absent or `Default`. `colour::cie_to_srgb` reads the entry and deliberately applies none of it.
//!
//! That decision's doc comment named *two* corpus pages as the only ones raising a black point, and
//! the four-hundred-and-sixty-first session measured eleven in the same two documents. This is the
//! command that counts them, so that no number has to be written down again.
//!
//! Every object the cross-reference table lists is scanned, rather than every space a page reaches:
//! a `/BlackPoint` inside an `/Indexed` base or a `/DeviceN` alternate is the same statement as one
//! in a page's own `/ColorSpace`, and the object graph reaches all of them without a walk that has
//! to know the shape of each.
//!
//! ```sh
//! cargo run --release -p pdf-model --example black_point_census
//! ```
#![expect(
    clippy::print_stdout,
    clippy::expect_used,
    reason = "a measurement whose output is its purpose, and which stops loudly where the corpus \
              is missing"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pdf_syntax::{Document, Object, ObjectId};

/// How far a colour space array is followed into another one.
const MAX_DEPTH: usize = 8;

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

/// The three families whose dictionary Table 62, Table 63 and Table 64 give a `/BlackPoint`.
fn is_cal_family(name: &[u8]) -> bool {
    matches!(name, b"CalGray" | b"CalRGB" | b"Lab")
}

/// A `/BlackPoint` as the file writes it, with whether any component is non-zero.
///
/// `None` where the entry is absent, which is Table 62's, Table 63's and Table 64's default of
/// `[0.0 0.0 0.0]` and is not the same statement as a file writing that default out.
fn black_point(document: &Document, dict: &pdf_syntax::Dictionary) -> Option<(String, bool)> {
    let Object::Array(items) = document.get_key(dict, "BlackPoint") else {
        return None;
    };
    let mut shown = Vec::new();
    let mut raised = false;
    for item in &items {
        match document.resolve(item) {
            Object::Integer(value) => {
                raised |= value != 0;
                shown.push(format!("{value}"));
            }
            Object::Real(value) => {
                raised |= value != 0.0;
                shown.push(format!("{value}"));
            }
            other => shown.push(format!("{other:?}")),
        }
    }
    Some((format!("[{}]", shown.join(" ")), raised))
}

/// Every Cal space inside one object, counted into `stated` and `raised`.
fn scan(
    document: &Document,
    object: &Object,
    depth: usize,
    stated: &mut BTreeMap<String, usize>,
    raised: &mut Vec<(String, String)>,
    file: &str,
) {
    if depth > MAX_DEPTH {
        return;
    }
    match object {
        Object::Array(items) => {
            if let Some(Object::Name(head)) = items.first()
                && is_cal_family(head.as_bytes())
                && let Some(dict) = items.get(1).map(|d| document.resolve(d))
                && let Some(dict) = dict.as_dict()
            {
                let family = String::from_utf8_lossy(head.as_bytes()).into_owned();
                if let Some((shown, non_zero)) = black_point(document, dict) {
                    let counter = stated.entry(family.clone()).or_default();
                    *counter = counter.saturating_add(1);
                    if non_zero {
                        raised.push((file.to_owned(), format!("/{family} {shown}")));
                    }
                }
            }
            for item in items {
                scan(
                    document,
                    item,
                    depth.saturating_add(1),
                    stated,
                    raised,
                    file,
                );
            }
        }
        Object::Dictionary(dict) => {
            for (_, value) in dict.iter() {
                scan(
                    document,
                    value,
                    depth.saturating_add(1),
                    stated,
                    raised,
                    file,
                );
            }
        }
        Object::Stream(stream) => {
            for (_, value) in stream.dict.iter() {
                scan(
                    document,
                    value,
                    depth.saturating_add(1),
                    stated,
                    raised,
                    file,
                );
            }
        }
        _ => {}
    }
}

fn main() {
    let mut documents = 0_usize;
    let mut stated: BTreeMap<String, usize> = BTreeMap::new();
    let mut raised: Vec<(String, String)> = Vec::new();

    for path in corpus() {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        documents = documents.saturating_add(1);
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let numbers: Vec<u32> = document.xref().object_numbers().collect();
        for number in numbers {
            let object = document.get(ObjectId::new(number, 0));
            scan(&document, &object, 0, &mut stated, &mut raised, &name);
        }
    }

    let total: usize = stated.values().copied().sum();
    println!("{documents} document(s) opened");
    println!("  {total} Cal space(s) state a /BlackPoint at all: {stated:?}");
    println!("  {} state one that is not [0 0 0]:", raised.len());
    for (file, what) in &raised {
        println!("    {file}  {what}");
    }
}
