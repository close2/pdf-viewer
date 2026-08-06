//! How many documents state §10.5's transfer function, and how many state a real one.
//!
//! Table 57's `/TR` and `/TR2` were on this tree's "describes a marking device" list until the
//! three-hundred-and-fifty-seventh session, when `issue6931_reduced.pdf` turned out to decide what
//! a *screen* shows with one. `doc/todo/13` said the number a round taking the clause owes first is
//! how many of the corpus state one at all, and how many state anything but `/Identity` — because
//! that decides whether it is one page or a population.
//!
//! Walks every page's `/Resources /ExtGState` and every form `XObject`'s, since §8.4.5's parameters
//! are set wherever a `gs` operator can name one.
//!
//! ```sh
//! cargo run --release -p pdf-model --example transfer_function_census -- doc/pdf.js/test/pdfs/*.pdf
//! ```

#![expect(
    clippy::print_stdout,
    reason = "an example whose entire output is a measurement"
)]

use std::collections::{BTreeMap, BTreeSet};

use pdf_syntax::{Dictionary, Document, Object};

/// How far a form `XObject`'s own resources are followed.
const MAX_DEPTH: usize = 8;

/// How many pages of one document are walked.
const MAX_PAGES: usize = 100;

fn main() {
    let mut documents = 0_usize;
    let mut stating = 0_usize;
    let mut stating_real = 0_usize;
    let mut states: BTreeMap<String, usize> = BTreeMap::new();
    let mut named: Vec<String> = Vec::new();
    let mut real: Vec<String> = Vec::new();

    for path in std::env::args().skip(1) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(document) = Document::open(bytes) else {
            continue;
        };
        documents = documents.saturating_add(1);
        let name = path.rsplit('/').next().unwrap_or(&path).to_owned();
        let pages = pdf_model::Pages::new(&document);
        let mut any = false;
        let mut any_real = false;
        for index in 0..pages.len().min(MAX_PAGES) {
            let Some(page) = pages.get(index) else {
                continue;
            };
            let mut seen = BTreeSet::new();
            walk(
                &document,
                &page.resources,
                0,
                &mut seen,
                &mut states,
                &mut any,
                &mut any_real,
            );
        }
        if any {
            stating = stating.saturating_add(1);
            named.push(name.clone());
        }
        if any_real {
            stating_real = stating_real.saturating_add(1);
            real.push(name);
        }
    }

    println!("{documents} document(s) opened");
    println!("  {stating} state a Table 57 /TR or /TR2 anywhere in a page's graphics states");
    println!("  **{stating_real} state one that is not /Identity or /Default**");
    println!("  values: {states:?}");
    if named.len() <= 20 {
        println!("  stating one: {}", named.join(" "));
    }
    println!("  stating a real one: {}", real.join(" "));
}

/// Every `/ExtGState` reachable from one resource dictionary, including through form `XObject`s.
fn walk(
    document: &Document,
    resources: &Dictionary,
    depth: usize,
    seen: &mut BTreeSet<pdf_syntax::ObjectId>,
    states: &mut BTreeMap<String, usize>,
    any: &mut bool,
    any_real: &mut bool,
) {
    if depth > MAX_DEPTH {
        return;
    }
    let graphics = document.get_key(resources, "ExtGState");
    if let Some(dict) = graphics.as_dict() {
        for (_, value) in dict.iter() {
            let resolved = document.resolve(value);
            let Some(state) = resolved.as_dict() else {
                continue;
            };
            for key in ["TR", "TR2"] {
                let entry = document.get_key(state, key);
                if entry.is_null() {
                    continue;
                }
                *any = true;
                let shape = match &entry {
                    Object::Name(name) => String::from_utf8_lossy(name.as_bytes()).into_owned(),
                    Object::Array(items) => format!("array of {}", items.len()),
                    Object::Dictionary(_) | Object::Stream(_) => "one function".to_owned(),
                    other => format!("{other:?}"),
                };
                if !matches!(shape.as_str(), "Identity" | "Default") {
                    *any_real = true;
                }
                let counter = states.entry(format!("/{key} {shape}")).or_default();
                *counter = counter.saturating_add(1);
            }
        }
    }
    let objects = document.get_key(resources, "XObject");
    let Some(dict) = objects.as_dict() else {
        return;
    };
    for (_, value) in dict.iter() {
        if let Some(id) = value.as_reference()
            && !seen.insert(id)
        {
            continue;
        }
        let resolved = document.resolve(value);
        let Object::Stream(stream) = &resolved else {
            continue;
        };
        let inner = document.get_key(&stream.dict, "Resources");
        if let Some(inner) = inner.as_dict() {
            walk(
                document,
                inner,
                depth.saturating_add(1),
                seen,
                states,
                any,
                any_real,
            );
        }
    }
}
